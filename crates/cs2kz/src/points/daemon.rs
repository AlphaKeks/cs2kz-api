use std::collections::HashMap;
use std::pin::pin;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;
use std::{io, mem};

use futures_util::{TryFutureExt as _, TryStreamExt as _};
use tokio::sync::Notify;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::Context;
use crate::database::{self, Database, QueryBuilder};
use crate::maps::CourseFilterId;
use crate::maps::courses::filters::GetCourseFiltersError;
use crate::python::Python;
use crate::records::GetRecordsError;

#[derive(Debug, Default)]
pub struct RecordCounts {
    counts: Mutex<HashMap<CourseFilterId, u64>>,
    notify: Notify,
}

impl RecordCounts {
    pub(crate) async fn new(database: &Database) -> database::Result<Self> {
        let counts =
            sqlx::query!("SELECT filter_id `filter_id: CourseFilterId`, `count` FROM RecordCounts")
                .fetch(database.as_ref())
                .map_ok(|row| (row.filter_id, row.count))
                .try_collect::<HashMap<_, _>>()
                .await?;

        Ok(Self {
            counts: Mutex::new(counts),
            notify: Notify::new(),
        })
    }

    pub fn increment(&self, filter: CourseFilterId) {
        self.counts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(filter)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        self.notify.notify_one();
    }

    async fn next_filter(&self) -> CourseFilterId {
        let mut notified = pin!(self.notify.notified());

        loop {
            notified.as_mut().enable();

            {
                let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);

                if let Some((&filter, &count)) = counts.iter().max_by_key(|&(_, &count)| count) {
                    tracing::debug!("chose {filter} with count {count}");
                    counts.remove(&filter);
                    break filter;
                }
            }

            notified.as_mut().await;
            notified.as_mut().set(self.notify.notified());
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    GetCourseFilter(GetCourseFiltersError),
    GetRecords(GetRecordsError),
    #[from(ignore)]
    GetCurrentRecordCounts(database::Error),
    #[from(ignore)]
    SaveFiltersToRecalculate(database::Error),
    Python(io::Error),
}

#[derive(Debug, serde::Serialize)]
struct PythonRequest {
    filter_id: CourseFilterId,
}

#[derive(Debug, serde::Deserialize)]
struct PythonResponse {
    #[expect(dead_code, reason = "included in tracing events")]
    filter_id: CourseFilterId,

    #[expect(dead_code, reason = "included in tracing events")]
    timings: PythonTimings,
}

#[derive(Debug, serde::Deserialize)]
#[expect(dead_code, reason = "included in tracing events")]
struct PythonTimings {
    #[serde(rename = "db_query_ms", deserialize_with = "deserialize_millis")]
    db_query: Duration,

    #[serde(rename = "nub_fit_ms", deserialize_with = "deserialize_millis")]
    nub_fit: Duration,

    #[serde(rename = "nub_compute_ms", deserialize_with = "deserialize_millis")]
    nub_compute: Duration,

    #[serde(rename = "pro_fit_ms", deserialize_with = "deserialize_millis")]
    pro_fit: Duration,

    #[serde(rename = "pro_compute_ms", deserialize_with = "deserialize_millis")]
    pro_compute: Duration,

    #[serde(rename = "db_write_ms", deserialize_with = "deserialize_millis")]
    db_write: Duration,
}

#[tracing::instrument(skip_all, err)]
pub async fn run(cx: Context, cancellation_token: CancellationToken) -> Result<(), Error> {
    let Some(script_path) = cx.config().points.calc_filter_path.as_deref() else {
        tracing::warn!("no `points.calc-filter-path` configured; points daemon will be disabled");
        return Ok(());
    };

    let mut python = Python::<PythonRequest, PythonResponse>::new(script_path.to_owned())?;

    loop {
        select! {
            () = cancellation_token.cancelled() => {
                warn!("saving record counts");
                update_record_counts(&cx).await?;
                save_filters_to_recalculate(&cx).await?;
                break Ok(());
            },

            filter_id = cx.record_counts().next_filter() => {
                process_filter(&mut python, &cancellation_token, filter_id).await?;
            },
        };
    }
}

#[tracing::instrument(skip(python))]
async fn process_filter(
    python: &mut Python<PythonRequest, PythonResponse>,
    cancellation_token: &CancellationToken,
    filter_id: CourseFilterId,
) -> Result<(), Error> {
    let request = PythonRequest { filter_id };

    loop {
        tracing::debug!(?request);
        match cancellation_token
            .run_until_cancelled(python.send_request(&request))
            .await
        {
            None => {
                tracing::debug!("cancelled");
                break Ok(());
            },
            Some(Ok(response)) => {
                tracing::debug!(?response);
                break Ok(());
            },
            Some(Err(err)) => {
                tracing::error!(%err, "failed to execute python request");
                python.reset().map_err(Error::Python)?;
                sleep(Duration::from_secs(1)).await;
            },
        }
    }
}

async fn update_record_counts(cx: &Context) -> Result<(), Error> {
    cx.database_transaction(async move |conn| {
        let mut counts = sqlx::query!(
            "SELECT * FROM (
               SELECT
                 record_id,
                 filter_id,
                 COUNT(*) OVER (PARTITION BY filter_id) AS count
               FROM BestNubRecords
             ) AS _
             GROUP BY filter_id",
        )
        .fetch(&mut *conn)
        .map_ok(|row| ((row.record_id, row.filter_id), row.count))
        .try_collect::<HashMap<_, _>>()
        .await?;

        {
            let mut pro_counts = sqlx::query!(
                "SELECT * FROM (
                   SELECT
                     record_id,
                     filter_id,
                     COUNT(*) OVER (PARTITION BY filter_id) AS count
                   FROM BestProRecords
                 ) AS _
                 GROUP BY filter_id",
            )
            .fetch(&mut *conn);

            while let Some(row) = pro_counts.try_next().await? {
                counts
                    .entry((row.record_id, row.filter_id))
                    .and_modify(|count| *count += row.count)
                    .or_insert(row.count);
            }
        }

        if counts.is_empty() {
            return Ok(());
        }

        sqlx::query!("DELETE FROM RecordCounts")
            .execute(&mut *conn)
            .await?;

        let mut query = QueryBuilder::new("INSERT INTO RecordCounts (filter_id, count) ");

        query.push_values(counts, |mut query, ((_, filter_id), count)| {
            query.push_bind(filter_id);
            query.push_bind(count);
        });

        query.push(" ON DUPLICATE KEY UPDATE count = VALUES(count)");

        query.build().persistent(false).execute(&mut *conn).await?;

        Ok(())
    })
    .map_err(Error::SaveFiltersToRecalculate)
    .await
}

async fn save_filters_to_recalculate(cx: &Context) -> Result<(), Error> {
    let counts = {
        let mut record_counts = cx
            .record_counts()
            .counts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        mem::take(&mut *record_counts)
    };

    if counts.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::new("INSERT INTO RecordCounts (filter_id, count) ");

    query.push_values(counts, |mut query, (filter_id, count)| {
        query.push_bind(filter_id);
        query.push_bind(count);
    });

    query.push(" ON DUPLICATE KEY UPDATE count = VALUES(count)");

    query
        .build()
        .persistent(false)
        .execute(cx.database().as_ref())
        .map_err(database::Error::from)
        .map_err(Error::SaveFiltersToRecalculate)
        .await?;

    Ok(())
}

fn deserialize_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <f64 as serde::Deserialize<'de>>::deserialize(deserializer)
        .map(|millis| millis / 1000.0)
        .map(Duration::from_secs_f64)
}
