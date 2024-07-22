//! A service for managing KZ maps.

use std::iter;

use axum::extract::FromRef;
use cs2kz::{GlobalStatus, SteamID};
use futures::{stream, StreamExt, TryFutureExt, TryStreamExt};
use itertools::Itertools;
use sqlx::{MySql, Pool, QueryBuilder, Transaction};
use tap::{Pipe, Tap, TryConv};

use crate::database::{FilteredQueryBuilder, SqlErrorExt};
use crate::services::SteamService;
use crate::util::MapIdentifier;

mod error;
pub use error::{Error, Result};

mod models;
pub use models::{
	CourseID,
	CreatedCourse,
	FetchMapRequest,
	FetchMapResponse,
	FetchMapsRequest,
	FetchMapsResponse,
	FilterID,
	MapID,
	NewCourse,
	NewFilter,
	SubmitMapRequest,
	SubmitMapResponse,
};

mod queries;
mod http;

/// A service for managing KZ maps.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct MapService
{
	database: Pool<MySql>,
	steam_svc: SteamService,
}

impl MapService
{
	/// Create a new [`MapService`].
	pub fn new(database: Pool<MySql>, steam_svc: SteamService) -> Self
	{
		Self { database, steam_svc }
	}

	/// Fetch a map.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_map(&self, req: FetchMapRequest) -> Result<Option<FetchMapResponse>>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		match req.ident {
			MapIdentifier::ID(map_id) => {
				query.push(" m.id = ").push_bind(map_id);
			}

			MapIdentifier::Name(map_name) => {
				query
					.push(" m.name LIKE ")
					.push_bind(format!("%{map_name}%"));
			}
		}

		let raw_maps = query
			.build_query_as::<FetchMapResponse>()
			.fetch_all(&self.database)
			.await?;

		let Some(map_id) = raw_maps.first().map(|m| m.id) else {
			return Ok(None);
		};

		let map = raw_maps
			.into_iter()
			.filter(|m| m.id == map_id)
			.reduce(reduce_chunk)
			.expect("we got the id we're filtering by from the original list");

		Ok(Some(map))
	}

	/// Fetch maps.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_maps(&self, req: FetchMapsRequest) -> Result<FetchMapsResponse>
	{
		let mut query = FilteredQueryBuilder::new(queries::SELECT);

		if let Some(name) = req.name {
			query.filter(" m.name LIKE ", format!("%{name}%"));
		}

		if let Some(workshop_id) = req.workshop_id {
			query.filter(" m.workshop_id = ", workshop_id);
		}

		if let Some(global_status) = req.global_status {
			query.filter(" m.global_status = ", global_status);
		}

		if let Some(created_after) = req.created_after {
			query.filter(" m.created_on > ", created_after);
		}

		if let Some(created_before) = req.created_before {
			query.filter(" m.created_on < ", created_before);
		}

		query.push(" ORDER BY m.id DESC ");

		let map_chunks = query
			.build_query_as::<FetchMapResponse>()
			.fetch_all(&self.database)
			.await?
			.into_iter()
			.chunk_by(|m| m.id);

		// Take into account how many maps we're gonna skip over
		let mut total = *req.offset;

		let maps = map_chunks
			.into_iter()
			.map(|(_, chunk)| chunk.reduce(reduce_chunk).expect("chunk can't be empty"))
			.skip(*req.offset as usize)
			.take(*req.limit as usize)
			.collect_vec();

		// Add all the maps we actually return
		total += maps.len() as u64;

		// And everything else that we would have ignored otherwise
		total += map_chunks.into_iter().count() as u64;

		Ok(FetchMapsResponse { maps, total })
	}

	/// Submit a new map.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn submit_map(&self, req: SubmitMapRequest) -> Result<SubmitMapResponse>
	{
		let mut txn = self.database.begin().await?;
		let (map_name, checksum) = tokio::try_join! {
			self.steam_svc
				.fetch_map_name(req.workshop_id)
				.map_err(Error::Steam),
			self.steam_svc
				.download_map(req.workshop_id)
				.map_err(Error::Steam)
				.and_then(|map_file| map_file.checksum().map_err(Error::CalculateMapChecksum)),
		}?;

		let map_id = create_map(&map_name, checksum, &req, &mut txn).await?;
		create_mappers(map_id, &req.mappers, &mut txn).await?;
		let courses = create_courses(map_id, &req.courses, &mut txn).await?;

		txn.commit().await?;

		Ok(SubmitMapResponse { map_id, courses })
	}

	/*
	/// Update an existing map.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn update_map(&self, req: UpdateMapRequest) -> Result<UpdateMapResponse>
	{
		todo!()
	}
	*/
}

/// Reduce function for merging multiple database results for the same map with
/// different mappers and courses.
///
/// When we fetch maps from the DB, we get "duplicates" for maps with multiple
/// mappers and/or courses, since SQL doesn't support arrays. All the
/// information in these results is the same, except for the mapper/course
/// information. We group results by their ID and then reduce each chunk down
/// into a single map using this function.
fn reduce_chunk(mut acc: FetchMapResponse, curr: FetchMapResponse) -> FetchMapResponse
{
	assert_eq!(acc.id, curr.id, "merging two unrelated maps");

	for mapper in curr.mappers {
		if !acc.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
			acc.mappers.push(mapper);
		}
	}

	for course in curr.courses {
		let Some(c) = acc.courses.iter_mut().find(|c| c.id == course.id) else {
			acc.courses.push(course);
			continue;
		};

		for mapper in course.mappers {
			if !c.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
				c.mappers.push(mapper);
			}
		}

		for filter in course.filters {
			if !c.filters.iter().any(|f| f.id == filter.id) {
				c.filters.push(filter);
			}
		}
	}

	acc
}

/// Creates a new map in the database and returns the generated ID.
async fn create_map(
	map_name: &str,
	checksum: u32,
	req: &SubmitMapRequest,
	txn: &mut Transaction<'_, MySql>,
) -> Result<MapID>
{
	let deglobal_result = sqlx::query! {
		r"
		UPDATE
		  Maps
		SET
		  global_status = ?
		WHERE
		  name = ?
		",
		GlobalStatus::NotGlobal,
		map_name,
	}
	.execute(txn.as_mut())
	.await?;

	match deglobal_result.rows_affected() {
		0 => { /* all good, this is a new map */ }
		1 => tracing::info! {
			target: "cs2kz_api::audit_log",
			%map_name,
			"degloballed old version of map",
		},
		amount => tracing::warn! {
			%map_name,
			%amount,
			"degloballed multiple old versions of map",
		},
	}

	let map_id = sqlx::query! {
		r"
		INSERT INTO
		  Maps (
		    name,
		    description,
		    global_status,
		    workshop_id,
		    checksum
		  )
		VALUES
		  (?, ?, ?, ?, ?)
		",
		map_name,
		req.description,
		req.global_status,
		req.workshop_id,
		checksum,
	}
	.execute(txn.as_mut())
	.await?
	.last_insert_id()
	.try_conv::<MapID>()
	.expect("in-range ID");

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		id = %map_id,
		name = %map_name,
		new = %(deglobal_result.rows_affected() == 0),
		"created map",
	};

	Ok(map_id)
}

/// Inserts mappers into the database.
async fn create_mappers(
	map_id: MapID,
	mapper_ids: &[SteamID],
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	QueryBuilder::new(queries::INSERT_MAPPERS)
		.tap_mut(|query| {
			query.push_values(mapper_ids, |mut query, mapper_id| {
				query.push_bind(map_id).push_bind(mapper_id);
			});
		})
		.build()
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("player_id") {
				Error::MapperDoesNotExist
			} else {
				Error::from(error)
			}
		})?;

	tracing::debug!(target: "cs2kz_api::audit_log", ?mapper_ids, "created mappers");

	Ok(())
}

async fn create_courses(
	map_id: MapID,
	courses: &[NewCourse],
	txn: &mut Transaction<'_, MySql>,
) -> Result<Vec<CreatedCourse>>
{
	QueryBuilder::new(queries::INSERT_COURSES)
		.tap_mut(|query| {
			query.push_values(courses, |mut query, course| {
				query.push_bind(course.name.as_deref());
				query.push_bind(course.description.as_deref());
				query.push_bind(map_id);
			});
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	let course_ids = sqlx::query_scalar! {
		r"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		",
	}
	.fetch_all(txn.as_mut())
	.await?;

	let mut created_courses = Vec::with_capacity(courses.len());

	for (id, course) in iter::zip(course_ids, courses) {
		insert_course_mappers(id, &course.mappers, txn).await?;
		insert_course_filters(id, &course.filters, txn)
			.await?
			.pipe(|filter_ids| created_courses.push(CreatedCourse { id, filter_ids }));
	}

	Ok(created_courses)
}

async fn insert_course_mappers(
	course_id: CourseID,
	mapper_ids: &[SteamID],
	txn: &mut Transaction<'_, MySql>,
) -> Result<()>
{
	QueryBuilder::new(queries::INSERT_COURSE_MAPPERS)
		.tap_mut(|query| {
			query.push_values(mapper_ids, |mut query, steam_id| {
				query.push_bind(course_id).push_bind(steam_id);
			});
		})
		.build()
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("player_id") {
				Error::MapperDoesNotExist
			} else {
				Error::from(error)
			}
		})?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?mapper_ids,
		"created course mappers",
	};

	Ok(())
}

async fn insert_course_filters(
	course_id: CourseID,
	filters: &[NewFilter; 4],
	txn: &mut Transaction<'_, MySql>,
) -> Result<[FilterID; 4]>
{
	QueryBuilder::new(queries::INSERT_COURSE_FILTERS)
		.tap_mut(|query| {
			query.push_values(filters, |mut query, filter| {
				query.push_bind(course_id);
				query.push_bind(filter.mode);
				query.push_bind(filter.teleports);
				query.push_bind(filter.tier);
				query.push_bind(filter.ranked_status);
				query.push_bind(filter.notes.as_deref());
			});
		})
		.build()
		.execute(txn.as_mut())
		.await?;

	let filter_ids = sqlx::query_scalar! {
		r"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		",
	}
	.fetch_all(txn.as_mut())
	.await?
	.try_conv::<[FilterID; 4]>()
	.expect("exactly 4 filters");

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?filter_ids,
		"created course filters",
	};

	Ok(filter_ids)
}
