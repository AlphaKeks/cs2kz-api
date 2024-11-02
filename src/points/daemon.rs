use std::collections::BTreeMap;

use cs2kz::Tier;
use futures::TryStreamExt;
use pyo3::{PyErr, PyResult};
use sqlx::{MySql, Pool};
use thiserror::Error;
use tokio::{select, task};
use tokio_util::sync::CancellationToken;

use crate::services::maps::FilterID;

struct Filter
{
	id: FilterID,
	tier: Tier,
	has_teleports: bool,
}

struct PointsData
{
	a: f64,
	b: f64,
	loc: f64,
	scale: f64,
	top_scale: f64,
}

#[derive(Debug, Error)]
enum Error
{
	#[error("python shit the bed: {0}")]
	Python(#[from] PyErr),

	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
}

pub fn run_daemon(pool: Pool<MySql>, shutdown_token: CancellationToken)
{
	task::spawn(async move {
		let mut data: Vec<(Filter, PointsData)> = sqlx::query!(
			"SELECT
			   cf.tier `tier: Tier`,
			   cf.teleports has_teleports,
			   p.* FROM PointsData p
			 JOIN CourseFilters cf ON cf.id = p.filter_id
			 JOIN Courses c ON c.id = cf.course_id
			 JOIN Maps m ON m.id = c.map_id
			 WHERE m.global_status = 1",
		)
		.fetch(&pool)
		.map_ok(|row| {
			let filter = Filter {
				id: FilterID::from(row.filter_id),
				tier: row.tier,
				has_teleports: row.has_teleports,
			};

			let data = PointsData {
				a: row.a,
				b: row.b,
				loc: row.loc,
				scale: row.scale,
				top_scale: row.top_scale,
			};

			(filter, data)
		})
		.try_collect()
		.await?;

		assert!(!data.is_empty(), "???");

		let mut data_iter = get_iter(&mut data);

		loop {
			let (filter, data) = loop {
				match data_iter.next() {
					Some(item) => break item,
					None => {
						drop(data_iter);
						data_iter = get_iter(&mut data);
					}
				}
			};

			select! {
				() = shutdown_token.cancelled() => break Ok::<_, Error>(()),
				Err(error) = do_the_thing(&pool, filter, data) => tracing::error!(%error, "whoops"),
				else => continue,
			}
		}
	});
}

fn get_iter(
	data: &mut Vec<(Filter, PointsData)>,
) -> impl Iterator<Item = (&Filter, &mut PointsData)>
{
	data.iter_mut().map(|&mut (ref k, ref mut v)| (k, v))
}

async fn do_the_thing(pool: &Pool<MySql>, filter: &Filter, data: &mut PointsData) -> PyResult<()>
{
	Ok(())
}
