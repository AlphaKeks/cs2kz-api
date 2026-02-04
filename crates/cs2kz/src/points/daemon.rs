use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures_util::TryFutureExt as _;
use tokio::sync::Notify;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::maps::CourseFilterId;
use crate::maps::courses::filters::GetCourseFiltersError;
use crate::mode::Mode;
use crate::players::PlayerId;
use crate::python::Python;
use crate::records::GetRecordsError;
use crate::{Context, database, players};

#[derive(Debug, Clone)]
pub struct PointsDaemonHandle {
    notifications: Arc<Notifications>,
}

impl PointsDaemonHandle {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Notifications { record_submitted: Notify::new() }),
        }
    }

    pub fn notify_record_submitted(&self) {
        self.notifications.record_submitted.notify_waiters();
    }
}

#[derive(Debug)]
struct Notifications {
    record_submitted: Notify,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    GetCourseFilter(GetCourseFiltersError),
    GetRecords(GetRecordsError),
    DetermineFilterToRecalculate(DetermineFilterToRecalculateError),
    DetermineRatingToRecalculate(DetermineRatingToRecalculateError),
    Python(io::Error),
}

#[derive(Debug, Display, Error, From)]
#[display("failed to determine next filter to recalculate: {_0}")]
#[from(forward)]
pub struct DetermineFilterToRecalculateError(database::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to determine next rating to recalculate: {_0}")]
#[from(forward)]
pub struct DetermineRatingToRecalculateError(database::Error);

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
        let Some(action) = determine_action(&cx).await? else {
            tracing::debug!("waiting for record to be submitted...");
            select! {
                () = cx.points_daemon().notifications.record_submitted.notified() => {
                    tracing::trace!("received notification about submitted record");
                    continue;
                },
                () = cancellation_token.cancelled() => {
                    tracing::debug!("cancelled");
                    break Ok(());
                },
            };
        };

        match action {
            Action::RecalcFilter(filter_id, priority) => {
                process_filter(&mut python, &cancellation_token, filter_id).await?;
                update_filters_to_recalculate(&cx, filter_id, priority).await;
            },
            Action::RecalcRating(player_id, mode, priority) => {
                if let Err(err) = players::update_rating(&cx, player_id, mode, priority).await {
                    tracing::error!(%err, %player_id, ?mode, priority, "failed to update player rating");
                }
            },
        }
    }
}

#[derive(Debug)]
enum Action {
    RecalcFilter(CourseFilterId, u64),
    RecalcRating(PlayerId, Mode, u64),
}

#[tracing::instrument(skip(cx))]
async fn determine_action(cx: &Context) -> Result<Option<Action>, Error> {
    if let Some((filter_id, priority)) = determine_filter_to_recalculate(cx).await? {
        return Ok(Some(Action::RecalcFilter(filter_id, priority)));
    }

    if let Some((player_id, mode, priority)) = determine_rating_to_recalculate(cx).await? {
        return Ok(Some(Action::RecalcRating(player_id, mode, priority)));
    }

    Ok(None)
}

#[tracing::instrument(skip(cx))]
async fn determine_filter_to_recalculate(
    cx: &Context,
) -> Result<Option<(CourseFilterId, u64)>, DetermineFilterToRecalculateError> {
    sqlx::query!(
        "SELECT
           filter_id AS `filter_id: CourseFilterId`,
           priority
         FROM FiltersToRecalculate
         WHERE priority > 0
         ORDER BY priority DESC
         LIMIT 1",
    )
    .fetch_optional(cx.database().as_ref())
    .map_ok(|maybe_row| maybe_row.map(|row| (row.filter_id, row.priority)))
    .map_err(DetermineFilterToRecalculateError::from)
    .await
}

#[tracing::instrument(skip(cx))]
async fn determine_rating_to_recalculate(
    cx: &Context,
) -> Result<Option<(PlayerId, Mode, u64)>, DetermineRatingToRecalculateError> {
    sqlx::query!(
        "SELECT
           player_id AS `player_id: PlayerId`,
           mode AS `mode: Mode`,
           priority
         FROM RatingsToRecalculate
         WHERE priority > 0
         ORDER BY priority DESC
         LIMIT 1",
    )
    .fetch_optional(cx.database().as_ref())
    .map_ok(|maybe_row| maybe_row.map(|row| (row.player_id, row.mode, row.priority)))
    .map_err(DetermineRatingToRecalculateError::from)
    .await
}

#[tracing::instrument(skip(cx))]
async fn update_filters_to_recalculate(
    cx: &Context,
    filter_id: CourseFilterId,
    prev_priority: u64,
) {
    if let Err(err) = sqlx::query!(
        "UPDATE FiltersToRecalculate
         SET priority = (priority - ?)
         WHERE filter_id = ?",
        prev_priority,
        filter_id,
    )
    .execute(cx.database().as_ref())
    .await
    {
        tracing::warn!(%err, %filter_id, prev_priority, "failed to update FiltersToRecalculate");
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

fn deserialize_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <f64 as serde::Deserialize<'de>>::deserialize(deserializer)
        .map(|millis| millis / 1000.0)
        .map(Duration::from_secs_f64)
}
