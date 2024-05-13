//! Handlers for the `/maps/{map}` route.

use std::collections::{BTreeMap, HashSet};

use axum::extract::Path;
use axum::Json;
use cs2kz::{GlobalStatus, MapIdentifier, Mode, SteamID};
use sqlx::{MySql, QueryBuilder, Transaction};
use tracing::{debug, info};

use super::root::{insert_course_mappers, insert_mappers};
use crate::authorization::Permissions;
use crate::database::{ResolveID, UpdateQuery};
use crate::http::{HandlerError, HandlerResult};
use crate::maps::{
	queries, CourseID, CourseUpdate, FilterID, FilterUpdate, FilterUpdates, FullMap, MapID,
	MapUpdate,
};
use crate::openapi::responses::NoContent;
use crate::openapi::{parameters, responses};
use crate::steam::workshop::{self, WorkshopID};
use crate::{authentication, authorization, Config, State};

/// Fetch a specific map by its name or ID.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Maps",
  path = "/maps/{map}",
  params(parameters::MapIdentifier),
  responses(
    responses::Ok<FullMap>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(state: &State, Path(map): Path<MapIdentifier>) -> HandlerResult<Json<FullMap>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE ");

	let map_id = map.resolve_id(&state.database).await?;

	query.push("m.id = ").push_bind(map_id);

	let map = query
		.build_query_as::<FullMap>()
		.fetch_all(&state.database)
		.await?
		.into_iter()
		.reduce(FullMap::merge)
		.ok_or_else(|| HandlerError::no_content())?;

	Ok(Json(map))
}

/// Create a new map.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  tag = "Maps",
  path = "/maps/{map_id}",
  params(parameters::MapID),
  request_body = MapUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn patch(
	state: &State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::MAPS.value() }>>,
	Path(map_id): Path<MapID>,
	Json(update): Json<MapUpdate>,
) -> HandlerResult<NoContent> {
	if update.is_empty() {
		return Ok(NoContent);
	}

	let mut transaction = state.database.begin().await?;

	update_map_metadata(
		map_id,
		update.description.as_deref(),
		update.global_status,
		update.workshop_id,
		&mut transaction,
	)
	.await?;

	if update.check_workshop || update.workshop_id.is_some() {
		check_workshop(
			map_id,
			update.workshop_id,
			&state.http_client,
			&state.config,
			&mut transaction,
		)
		.await?;
	}

	if let Some(mappers) = update.added_mappers {
		insert_mappers(map_id, &mappers, &mut transaction).await?;
	}

	if let Some(mappers) = update.removed_mappers {
		delete_mappers(map_id, &mappers, &mut transaction).await?;
	}

	if let Some(updates) = update.course_updates {
		update_courses(map_id, &updates, &mut transaction).await?;
	}

	transaction.commit().await?;

	info!(target: "audit_log", %map_id, "updated map");

	Ok(NoContent)
}

/// Updates metadata about a map in the database.
async fn update_map_metadata(
	map_id: MapID,
	description: Option<&str>,
	global_status: Option<GlobalStatus>,
	workshop_id: Option<WorkshopID>,
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<()> {
	if description.is_none() && global_status.is_none() && workshop_id.is_none() {
		return Ok(());
	}

	let mut query = UpdateQuery::new("Maps");

	if let Some(description) = description {
		query.set("description", description);
	}

	if let Some(global_status) = global_status {
		query.set("global_status", global_status);
	}

	if let Some(workshop_id) = workshop_id {
		query.set("workshop_id", workshop_id);
	}

	query.push(" WHERE id = ").push_bind(map_id);

	let was_updated = query
		.build()
		.execute(transaction.as_mut())
		.await
		.map(|result| result.rows_affected() > 0)?;

	if !was_updated {
		return Err(HandlerError::unknown("map"));
	}

	debug!(target: "audit_log", %map_id, "updated map metadata");

	Ok(())
}

/// Checks the Steam Workshop if a given map has changed its name or file contents.
async fn check_workshop(
	map_id: MapID,
	workshop_id: Option<WorkshopID>,
	http_client: &reqwest::Client,
	config: &Config,
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<()> {
	let workshop_id = if let Some(id) = workshop_id {
		id
	} else {
		sqlx::query_scalar! {
			r#"
			SELECT
			  workshop_id `workshop_id: WorkshopID`
			FROM
			  Maps
			WHERE
			  id = ?
			"#,
			map_id,
		}
		.fetch_one(transaction.as_mut())
		.await?
	};

	let (name, checksum) =
		workshop::fetch_and_download_map(workshop_id, http_client, config).await?;

	let was_updated = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  name = ?,
		  CHECKSUM = ?
		WHERE
		  id = ?
		"#,
		name,
		checksum,
		map_id,
	}
	.execute(transaction.as_mut())
	.await
	.map(|result| result.rows_affected() > 0)?;

	if !was_updated {
		return Err(HandlerError::unknown("map"));
	}

	debug!(target: "audit_log", %map_id, "synced map with workshop");

	Ok(())
}

/// Deletes mappers for a specific map.
///
/// # Panics
///
/// This function will panic if `mappers` is empty.
async fn delete_mappers(
	map_id: MapID,
	mappers: &[SteamID],
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<()> {
	assert!(!mappers.is_empty(), "shouldn't try to delete 0 mappers");

	let mut query = QueryBuilder::new("DELETE FROM Mappers WHERE map_id = ");

	query.push_bind(map_id).push(" AND player_id IN (");

	let mut separated = query.separated(", ");

	for &steam_id in mappers {
		separated.push_bind(steam_id);
	}

	query.push(")");
	query.build().execute(transaction.as_mut()).await?;

	let remaining_mappers = sqlx::query_scalar! {
		r#"
		SELECT
		  COUNT(*)
		FROM
		  Mappers
		WHERE
		  map_id = ?
		"#,
		map_id,
	}
	.fetch_one(transaction.as_mut())
	.await?;

	if remaining_mappers == 0 {
		return Err(HandlerError::map_must_have_mappers());
	}

	debug!(target: "audit_log", %map_id, ?mappers, "deleted mappers for map");

	Ok(())
}

/// Updates courses.
///
/// # Panics
///
/// This function will panic if `updates` is empty.
async fn update_courses(
	map_id: MapID,
	updates: &BTreeMap<CourseID, CourseUpdate>,
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<()> {
	assert!(!updates.is_empty(), "shouldn't try to update 0 courses");

	let mut valid_course_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  map_id = ?
		"#,
		map_id,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.collect::<HashSet<_>>();

	let updates = updates.iter().map(|(&id, update)| {
		if valid_course_ids.remove(&id) {
			Ok((id, update))
		} else {
			Err(HandlerError::course_does_not_belong_to_map(id, map_id))
		}
	});

	let mut updated_course_ids = Vec::with_capacity(updates.len());

	for update in updates {
		let (course_id, update) = update?;

		if let Some(course_id) = update_course(course_id, update, transaction).await? {
			updated_course_ids.push(course_id);
		}
	}

	updated_course_ids.sort_unstable();

	debug!(target: "audit_log", %map_id, ?updated_course_ids, "updated courses");

	Ok(())
}

/// Updates a single course.
async fn update_course(
	course_id: CourseID,
	update: &CourseUpdate,
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<Option<CourseID>> {
	if update.is_empty() {
		return Ok(None);
	}

	if update.name.is_some() || update.description.is_some() {
		let mut query = UpdateQuery::new("Courses");

		if let Some(name) = &update.name {
			query.set("name", name.as_deref());
		}

		if let Some(description) = &update.description {
			query.set("description", description.as_deref());
		}

		query.push(" WHERE id = ").push_bind(course_id);
		query.build().execute(transaction.as_mut()).await?;
	}

	if let Some(mappers) = update.added_mappers.as_deref() {
		insert_course_mappers(course_id, mappers, transaction).await?;
	}

	if let Some(mappers) = update.removed_mappers.as_deref() {
		delete_course_mappers(course_id, mappers, transaction).await?;
	}

	if let Some(updates) = update.filter_updates.as_ref() {
		update_filters(course_id, updates, transaction).await?;
	}

	Ok(Some(course_id))
}

/// Deletes mappers for a specific course.
///
/// # Panics
///
/// This function will panic if `mappers` is empty.
async fn delete_course_mappers(
	course_id: CourseID,
	mappers: &[SteamID],
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<()> {
	assert!(!mappers.is_empty(), "shouldn't try to delete 0 mappers");

	let mut query = QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ");

	query.push_bind(course_id).push(" AND player_id IN (");

	let mut separated = query.separated(", ");

	for &steam_id in mappers {
		separated.push_bind(steam_id);
	}

	query.push(")");
	query.build().execute(transaction.as_mut()).await?;

	let remaining_mappers = sqlx::query_scalar! {
		r#"
		SELECT
		  COUNT(*)
		FROM
		  CourseMappers
		WHERE
		  course_id = ?
		"#,
		course_id,
	}
	.fetch_one(transaction.as_mut())
	.await?;

	if remaining_mappers == 0 {
		return Err(HandlerError::course_must_have_mappers(course_id));
	}

	debug!(target: "audit_log", %course_id, ?mappers, "deleted mappers for course");

	Ok(())
}

/// Updates filters of a specific course.
///
/// # Panics
///
/// This function will panic if:
///    - `updates` is empty
///    - there is an incorrect amount of filters in the database (bug)
async fn update_filters(
	course_id: CourseID,
	updates: &FilterUpdates,
	transaction: &mut Transaction<'_, MySql>,
) -> HandlerResult<()> {
	assert!(!updates.is_empty(), "shouldn't try to update 0 filters");

	let mut valid_filter_ids = sqlx::query! {
		r#"
		SELECT
		  id `id: FilterID`,
		  mode `mode: Mode`,
		  teleports `teleports: bool`
		FROM
		  CourseFilters
		WHERE
		  course_id = ?
		"#,
		course_id,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.map(|row| ((row.mode, row.teleports), row.id))
	.collect::<BTreeMap<_, _>>();

	assert_eq!(
		valid_filter_ids.len(),
		4,
		"every course should have 4 filters"
	);

	let updates = updates.iter().map(|(&filter, update)| {
		if let Some(filter_id) = valid_filter_ids.remove(&filter) {
			Ok((filter_id, update))
		} else {
			Err(HandlerError::duplicate_filter(filter.0, filter.1))
		}
	});

	let mut updated_filter_ids = Vec::with_capacity(updates.len());

	for update in updates {
		let (filter_id, update) = update?;

		if let Some(filter_id) = update_filter(filter_id, update, transaction).await? {
			updated_filter_ids.push(filter_id);
		}
	}

	updated_filter_ids.sort_unstable();

	debug!(target: "audit_log", %course_id, ?updated_filter_ids, "updated course filters");

	Ok(())
}

/// Updates a single course filter.
async fn update_filter(
	filter_id: FilterID,
	update: &FilterUpdate,
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<Option<FilterID>> {
	if update.is_empty() {
		return Ok(None);
	}

	let mut query = UpdateQuery::new("CourseFilters");

	if let Some(tier) = update.tier {
		query.set("tier", tier);
	}

	if let Some(ranked_status) = update.ranked_status {
		query.set("ranked_status", ranked_status);
	}

	if let Some(notes) = &update.notes {
		query.set("notes", notes.as_deref());
	}

	query.push(" WHERE id = ").push_bind(filter_id);
	query.build().execute(transaction.as_mut()).await?;

	Ok(Some(filter_id))
}
