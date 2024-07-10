//! Everything related to KZ records.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::sync::Arc;

use axum::extract::FromRef;
use cs2kz::Styles;
use sqlx::{MySql, Pool, QueryBuilder};

use crate::authentication::JwtState;
use crate::maps::FilterID;
use crate::sqlx::query::QueryBuilderExt;
use crate::sqlx::{query, FetchID, FilteredQuery, SqlErrorExt};
use crate::{authentication, Error, Result};

mod models;
pub use models::{
	BhopStats,
	CreatedRecord,
	FetchRecordsRequest,
	NewRecord,
	Record,
	RecordID,
	SortRecordsBy,
};

mod queries;
pub mod http;

/// A service for dealing with KZ records as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct RecordService
{
	database: Pool<MySql>,
	jwt_state: Arc<JwtState>,
}

impl RecordService
{
	/// Creates a new [`RecordService`] instance.
	pub const fn new(database: Pool<MySql>, jwt_state: Arc<JwtState>) -> Self
	{
		Self { database, jwt_state }
	}

	/// Fetches a single record.
	pub async fn fetch_record(&self, record_id: RecordID) -> Result<Record>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push(" WHERE r.id = ").push_bind(record_id);

		let record = query
			.build_query_as::<Record>()
			.fetch_optional(&self.database)
			.await?
			.ok_or_else(|| Error::not_found("record"))?;

		Ok(record)
	}

	/// Fetches many records.
	pub async fn fetch_records(&self, request: FetchRecordsRequest) -> Result<(Vec<Record>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = FilteredQuery::new(queries::SELECT);

		if let Some(mode) = request.mode {
			query.filter(" f.mode_id = ", mode);
		}

		if request.styles != Styles::NONE {
			query
				.filter(" ((r.styles & ", request.styles)
				.push(") = ")
				.push_bind(request.styles)
				.push(")");
		}

		match request.teleports {
			None => {}
			Some(true) => {
				query.filter(" r.teleports > ", 0);
			}
			Some(false) => {
				query.filter(" r.teleports = ", 0);
			}
		}

		if let Some(player) = request.player {
			let steam_id = player.fetch_id(transaction.as_mut()).await?;

			query.filter(" r.player_id = ", steam_id);
		}

		if let Some(map) = request.map {
			let map_id = map.fetch_id(transaction.as_mut()).await?;

			query.filter(" m.id = ", map_id);
		}

		if let Some(course) = request.course {
			let course_id = course.fetch_id(transaction.as_mut()).await?;

			query.filter(" c.id = ", course_id);
		}

		if let Some(server) = request.server {
			let server_id = server.fetch_id(transaction.as_mut()).await?;

			query.filter(" r.server_id = ", server_id);
		}

		if let Some(created_after) = request.created_after {
			query.filter(" r.created_on > ", created_after);
		}

		if let Some(created_before) = request.created_before {
			query.filter(" r.created_on < ", created_before);
		}

		query.order_by(request.sort_order, match request.sort_by {
			SortRecordsBy::Time => "r.time",
			SortRecordsBy::Date => "r.created_on",
		});

		query.push_limits(request.limit, request.offset);

		let records = query
			.build_query_as::<Record>()
			.fetch_all(transaction.as_mut())
			.await?;

		if records.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((records, total))
	}

	/// Submits a new record.
	pub async fn submit_record(
		&self,
		record: NewRecord,
		server: authentication::Server,
	) -> Result<CreatedRecord>
	{
		let mut transaction = self.database.begin().await?;

		let filter_id = sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: FilterID`
			FROM
			  CourseFilters
			WHERE
			  course_id = ?
			  AND mode_id = ?
			  AND teleports = ?
			"#,
			record.course_id,
			record.mode,
			record.teleports > 0,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or_else(|| Error::not_found("course"))?;

		let record_id = sqlx::query! {
			r#"
			INSERT INTO
			  Records (
			    filter_id,
			    styles,
			    teleports,
			    time,
			    player_id,
			    server_id,
			    bhops,
			    perfs,
			    plugin_version_id
			  )
			VALUES
			  (?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
			filter_id,
			record.styles,
			record.teleports,
			record.time.as_secs_f64(),
			record.player_id,
			server.id(),
			record.bhop_stats.bhops,
			record.bhop_stats.perfs,
			server.plugin_version_id(),
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::not_found("player").context(err)
			} else {
				Error::from(err)
			}
		})?
		.last_insert_id()
		.into();

		transaction.commit().await?;

		tracing::trace!(%record_id, "created record");

		Ok(CreatedRecord { record_id })
	}
}
