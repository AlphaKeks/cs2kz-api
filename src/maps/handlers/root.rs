//! Handlers for the `/maps` route.

use std::collections::BTreeMap;
use std::iter;

use axum::Json;
use axum_extra::extract::Query;
use chrono::{DateTime, Utc};
use cs2kz::{GlobalStatus, SteamID};
use query::QueryBuilderExt;
use serde::Deserialize;
use sqlx::{MySql, QueryBuilder, Transaction};
use tracing::{info, warn};
use utoipa::IntoParams;

use crate::authorization::Permissions;
use crate::database::{query, FilteredQuery, SqlxErrorExt};
use crate::http::{HandlerError, HandlerResult, Pagination};
use crate::maps::{
	queries, CourseID, CreatedMap, FilterID, FullMap, MapID, NewCourse, NewFilter, NewMap,
};
use crate::openapi::parameters::{Limit, Offset, SortingOrder};
use crate::openapi::responses::{self, Created};
use crate::steam::workshop::{self, WorkshopID};
use crate::{authentication, authorization, State};

/// Query parameters for `GET /maps`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by map name.
	name: Option<String>,

	/// Filter by workshop ID.
	workshop_id: Option<WorkshopID>,

	/// Filter by global status.
	global_status: Option<GlobalStatus>,

	/// Only include maps approved after this date.
	created_after: Option<DateTime<Utc>>,

	/// Only include maps approved before this date.
	created_before: Option<DateTime<Utc>>,

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Fetch maps.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Maps",
  path = "/maps",
  params(GetParams),
  responses(
    responses::Ok<Pagination<FullMap>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: &State,
	Query(GetParams {
		name,
		workshop_id,
		global_status,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> HandlerResult<Json<Pagination<FullMap>>> {
	let mut transaction = state.database.begin().await?;
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(name) = name.filter(|s| !s.is_empty()) {
		query.filter("m.name LIKE ", format!("%{name}%"));
	}

	if let Some(workshop_id) = workshop_id {
		query.filter("m.workshop_id = ", workshop_id);
	}

	if let Some(global_status) = global_status {
		query.filter("m.global_status = ", global_status);
	}

	if let Some(created_after) = created_after {
		query.filter("m.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter("m.created_on < ", created_before);
	}

	// As long as IDs don't have holes in them, this should work, right?
	//
	// :clueless:
	if let offset @ 1.. = offset.0 {
		query.filter("m.id > ", offset);
	}

	query.order_by(SortingOrder::Ascending, "m.id");

	let maps = query
		.build_query_as::<FullMap>()
		.fetch_all(transaction.as_mut())
		.await
		.map(|maps| FullMap::normalize_sql_results(maps, limit.into()))?;

	if maps.is_empty() {
		return Err(HandlerError::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(Pagination::new(total, maps)))
}

/// Create a new map.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  put,
  tag = "Maps",
  path = "/maps",
  request_body = NewMap,
  responses(
    responses::Created<CreatedMap>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn put(
	state: &State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::MAPS.value() }>>,
	Json(NewMap {
		description,
		global_status,
		workshop_id,
		mappers,
		courses,
	}): Json<NewMap>,
) -> HandlerResult<Created<Json<CreatedMap>>> {
	let (name, checksum) =
		workshop::fetch_and_download_map(workshop_id, &state.http_client, &state.config).await?;

	let mut transaction = state.database.begin().await?;

	let map_id = insert_map(
		&name,
		description.as_deref(),
		global_status,
		workshop_id,
		checksum,
		&mut transaction,
	)
	.await?;

	let handle_missing_mappers = |err: sqlx::Error| {
		if err.is_fk_violation("player_id") {
			HandlerError::unknown("mapper").with_source(err)
		} else {
			HandlerError::from(err)
		}
	};

	insert_mappers(map_id, &mappers, &mut transaction)
		.await
		.map_err(handle_missing_mappers)?;

	let (course_ids, filter_ids) = insert_courses(map_id, &courses, &mut transaction)
		.await
		.map_err(handle_missing_mappers)?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedMap {
		map_id,
		course_ids,
		filter_ids,
	})))
}

/// Inserts a map into the database.
///
/// # Panics
///
/// This function might panic if the database ever returns an invalid ID.
async fn insert_map(
	name: &str,
	description: Option<&str>,
	global_status: GlobalStatus,
	workshop_id: WorkshopID,
	checksum: u32,
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<MapID> {
	let deglobal_result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  global_status = 'not_global'
		WHERE
		  name = ?
		"#,
		name,
	}
	.execute(transaction.as_mut())
	.await?;

	match deglobal_result.rows_affected() {
		0 => {}
		1 => {
			info!(target: "audit_log", %name, "degloballed old version of map");
		}
		n => {
			warn!(target: "audit_log", %name, amount = %n, "degloballed multiple versions of map");
		}
	}

	let map_id = sqlx::query! {
		r#"
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
		"#,
		name,
		description,
		global_status,
		workshop_id,
		checksum,
	}
	.execute(transaction.as_mut())
	.await?
	.last_insert_id()
	.try_into()
	.map(MapID)
	.expect("valid ID");

	info!(target: "audit_log", id = %map_id, %name, "created new map");

	Ok(map_id)
}

/// Inserts mappers into the database.
///
/// # Panics
///
/// This function will panic if the `mappers` slice is ever empty (bug).
pub(super) async fn insert_mappers(
	map_id: MapID,
	mappers: &[SteamID],
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<()> {
	assert!(!mappers.is_empty(), "can't have 0 mappers for map");

	let mut query = QueryBuilder::new("INSERT INTO Mappers (map_id, player_id)");

	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(map_id).push_bind(steam_id);
	});

	query.build().execute(transaction.as_mut()).await?;

	info!(target: "audit_log", %map_id, ?mappers, "created mappers");

	Ok(())
}

/// Inserts courses into the database.
///
/// # Panics
///
/// This function will panic if:
///    - the passed in `courses` slice is empty
///    - there is a bug with how we compute the course IDs
async fn insert_courses(
	map_id: MapID,
	courses: &[NewCourse],
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<(Vec<CourseID>, BTreeMap<CourseID, [FilterID; 4]>)> {
	assert!(!courses.is_empty(), "can't have 0 courses for map");

	let mut query = QueryBuilder::new("INSERT INTO Courses (name, description, map_id)");

	query.push_values(courses, |mut query, course| {
		query
			.push_bind(course.name.as_deref())
			.push_bind(course.description.as_deref())
			.push_bind(map_id);
	});

	let first_course_id = query
		.build()
		.execute(transaction.as_mut())
		.await?
		.last_insert_id();

	let course_ids = (first_course_id..)
		.take(courses.len())
		.map(|id| id.try_into().map(CourseID))
		.collect::<Result<Vec<_>, _>>()
		.expect("valid course IDs");

	info!(target: "audit_log", %map_id, ?course_ids, "created courses");

	let mut filter_ids = BTreeMap::new();

	for (&course_id, course) in iter::zip(&course_ids, courses) {
		insert_course_mappers(course_id, &course.mappers, transaction).await?;
		insert_course_filters(course_id, &course.filters, transaction)
			.await
			.map(|ids| filter_ids.insert(course_id, ids))?;
	}

	Ok((course_ids, filter_ids))
}

/// Inserts mappers for a specific course into the database.
///
/// # Panics
///
/// This function will panic if the `mappers` slice is empty.
pub(super) async fn insert_course_mappers(
	course_id: CourseID,
	mappers: &[SteamID],
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<()> {
	assert!(!mappers.is_empty(), "can't have 0 mappers for course");

	let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(course_id).push_bind(steam_id);
	});

	query.build().execute(transaction.as_mut()).await?;

	info!(target: "audit_log", %course_id, ?mappers, "created course mappers");

	Ok(())
}

/// Inserts filters for a specific course into the database.
///
/// # Panics
///
/// This function will panic if:
///    - the passed in `filters` slice is empty
///    - there is a bug with how we compute the filter IDs
async fn insert_course_filters(
	course_id: CourseID,
	filters: &[NewFilter; 4],
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<[FilterID; 4]> {
	let mut query = QueryBuilder::new(
		r#"
		INSERT INTO
		  CourseFilters (
		    course_id,
		    mode,
		    teleports,
		    tier,
		    ranked_status,
		    notes
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?)
		"#,
	);

	query.push_values(filters, |mut query, filter| {
		query
			.push_bind(course_id)
			.push_bind(filter.mode)
			.push_bind(filter.teleports)
			.push_bind(filter.tier)
			.push_bind(filter.ranked_status)
			.push_bind(filter.notes.as_deref());
	});

	let first_filter_id = query
		.build()
		.execute(transaction.as_mut())
		.await?
		.last_insert_id();

	let filter_ids = (first_filter_id..)
		.take(4)
		.map(|id| id.try_into().map(FilterID))
		.collect::<Result<Vec<_>, _>>()
		.expect("valid filter IDs")
		.try_into()
		.expect("we took exactly 4");

	Ok(filter_ids)
}
