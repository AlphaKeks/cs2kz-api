use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::Path;
use axum::Json;
use cs2kz::{SteamID, Tier};
use itertools::Itertools;
use sqlx::{MySql, MySqlExecutor, QueryBuilder, Transaction};

use crate::database::{GlobalStatus, RankedStatus};
use crate::extractors::State;
use crate::maps::{CourseUpdate, FilterUpdate, MapUpdate, MappersTable};
use crate::steam::workshop;
use crate::{query, responses, Error, Result};

/// Update a map with non-breaking changes.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Maps",
  path = "/maps/{map_id}",
  params(("map_id" = u16, Path, description = "The map's ID")),
  request_body = MapUpdate,
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["maps_edit", "maps_deglobal"]),
  ),
)]
pub async fn update(
	state: State,
	Path(map_id): Path<u16>,
	Json(map_update): Json<MapUpdate>,
) -> Result<()> {
	let mut transaction = state.transaction().await?;

	validate_update(map_id, &map_update, &mut transaction).await?;

	if let Some(global_status) = map_update.global_status {
		update_global_status(map_id, global_status, transaction.as_mut()).await?;
	}

	if let Some(workshop_id) = map_update.workshop_id {
		update_workshop_id(map_id, workshop_id, state.http(), &mut transaction).await?;
	}

	if let Some(mappers) = &map_update.added_mappers {
		super::create::insert_mappers(MappersTable::Map(map_id), mappers, transaction.as_mut())
			.await?;
	}

	if let Some(mappers) = &map_update.removed_mappers {
		remove_mappers(MappersTable::Map(map_id), mappers, transaction.as_mut()).await?;
	}

	if let Some(course_ids) = &map_update.removed_courses {
		remove_courses(course_ids, transaction.as_mut()).await?;
	}

	for course_update in map_update.course_updates.iter().flatten() {
		update_course(course_update, &mut transaction).await?;
	}

	transaction.commit().await?;

	Ok(())
}

async fn validate_update(
	map_id: u16,
	map_update: &MapUpdate,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	let course_ids = sqlx::query! {
		r#"
		SELECT
		  c.id
		FROM
		  Courses c
		  JOIN Maps m ON m.id = c.map_id
		WHERE
		  m.id = ?
		"#,
		map_id,
	}
	.fetch_all(transaction.as_mut())
	.await?
	.into_iter()
	.map(|row| row.id)
	.collect::<HashSet<u32>>();

	if course_ids.is_empty() {
		return Err(Error::UnknownMapID(map_id));
	}

	let removed_courses = map_update.removed_courses.iter().flatten().copied();
	let course_updates = map_update
		.course_updates
		.iter()
		.flatten()
		.map(|course| course.id);

	if let Some(course_id) = removed_courses
		.chain(course_updates)
		.find(|course_id| !course_ids.contains(course_id))
	{
		return Err(Error::InvalidCourse { map_id, course_id });
	}

	for course_update in map_update.course_updates.iter().flatten() {
		let filter_ids = sqlx::query! {
			r#"
			SELECT
			  f.id
			FROM
			  CourseFilters f
			  JOIN Courses c ON c.id = f.course_id
			WHERE
			  c.id = ?
			"#,
			course_update.id,
		}
		.fetch_all(transaction.as_mut())
		.await?;

		if let Some(filter) = course_update
			.filter_updates
			.iter()
			.flatten()
			.find(|filter| filter_ids.iter().map(|row| row.id).contains(&filter.id))
		{
			return Err(Error::InvalidFilter { course_id: course_update.id, filter_id: filter.id });
		}
	}

	Ok(())
}

async fn update_global_status(
	map_id: u16,
	global_status: GlobalStatus,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	sqlx::query! {
		r#"
			UPDATE
			  Maps
			SET
			  global_status = ?
			WHERE
			  id = ?
			"#,
		global_status,
		map_id,
	}
	.execute(executor)
	.await?;

	Ok(())
}

async fn update_workshop_id(
	map_id: u16,
	workshop_id: u32,
	http_client: Arc<reqwest::Client>,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	sqlx::query! {
		r#"
		UPDATE
		Maps
		SET
		workshop_id = ?
		WHERE
		id = ?
		"#,
		workshop_id,
		map_id,
	}
	.execute(transaction.as_mut())
	.await?;

	let (workshop_map, checksum) = tokio::try_join! {
		workshop::Map::get(workshop_id, http_client),
		async { workshop::MapFile::download(workshop_id).await?.checksum().await },
	}?;

	sqlx::query! {
		r#"
		UPDATE
		Maps
		SET
		name = ?,
		checksum = ?
		WHERE
		id = ?
		"#,
		workshop_map.name,
		checksum,
		map_id,
	}
	.execute(transaction.as_mut())
	.await?;

	Ok(())
}

async fn remove_mappers(
	table: MappersTable,
	mappers: &[SteamID],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let mut query = QueryBuilder::new("DELETE FROM ");

	match table {
		MappersTable::Map(map_id) => {
			query.push("Mappers WHERE map_id = ").push_bind(map_id);
		}
		MappersTable::Course(course_id) => {
			query
				.push("CourseMappers WHERE course_id = ")
				.push_bind(course_id);
		}
	}

	query.push(" AND player_id IN ");
	query::push_tuple(mappers, &mut query);

	query.build().execute(executor).await?;

	Ok(())
}

async fn remove_courses(course_ids: &[u32], executor: impl MySqlExecutor<'_>) -> Result<()> {
	let mut query = QueryBuilder::new("DELETE FROM Courses WHERE id IN");

	query::push_tuple(course_ids, &mut query);

	query.build().execute(executor).await?;

	Ok(())
}

async fn update_course(
	update: &CourseUpdate,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	if let Some(mappers) = &update.added_mappers {
		super::create::insert_mappers(
			MappersTable::Course(update.id),
			mappers,
			transaction.as_mut(),
		)
		.await?;
	}

	if let Some(mappers) = &update.removed_mappers {
		remove_mappers(MappersTable::Course(update.id), mappers, transaction.as_mut()).await?;
	}

	for FilterUpdate { id, tier, ranked_status } in update.filter_updates.iter().flatten().copied()
	{
		if tier.is_none() && ranked_status.is_none() {
			continue;
		}

		if tier.is_some_and(|tier| tier > Tier::Death)
			&& matches!(ranked_status, Some(RankedStatus::Ranked))
		{
			return Err(Error::UnrankableFilterWithID { id });
		}

		let mut query = QueryBuilder::new("UPDATE CourseFilters");
		let mut delimiter = " SET ";

		if let Some(tier) = tier {
			query.push(delimiter).push(" tier = ").push_bind(tier);

			delimiter = ",";
		}

		if let Some(ranked_status) = ranked_status {
			query
				.push(delimiter)
				.push(" ranked_status = ")
				.push_bind(ranked_status);
		}

		query.push(" WHERE id = ").push_bind(id);
		query.build().execute(transaction.as_mut()).await?;
	}

	Ok(())
}