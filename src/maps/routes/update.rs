use std::collections::HashSet;
use std::num::NonZeroU32;

use axum::extract::Path;
use axum::Json;
use cs2kz::{SteamID, Tier};
use itertools::Itertools;
use serde_json::json;
use sqlx::{MySql, MySqlExecutor, QueryBuilder, Transaction};
use tracing::trace;

use crate::database::{GlobalStatus, RankedStatus};
use crate::maps::{CourseUpdate, FilterUpdate, MapUpdate, MappersTable};
use crate::responses::NoContent;
use crate::steam::workshop;
use crate::{audit, query, responses, AppState, Error, Result};

/// Update metadata for a map.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Maps",
  path = "/maps/{map_id}",
  params(("map_id" = u16, Path, description = "The map's ID")),
  request_body = MapUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
    responses::BadGateway,
  ),
  security(
    ("Steam Session" = ["maps"]),
  ),
)]
pub async fn update(
	state: AppState,
	Path(map_id): Path<u16>,
	Json(map_update): Json<MapUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.begin_transaction().await?;

	validate_update(map_id, &map_update, &mut transaction).await?;

	if let Some(global_status) = map_update.global_status {
		update_global_status(map_id, global_status, transaction.as_mut()).await?;
	}

	if let Some(description) = map_update.description {
		update_description(map_id, &description, transaction.as_mut()).await?;
	}

	let workshop_id = if let Some(workshop_id) = map_update.workshop_id {
		update_workshop_id(map_id, workshop_id, transaction.as_mut()).await?;
		workshop_id
	} else {
		sqlx::query!("SELECT workshop_id FROM Maps WHERE id = ?", map_id)
			.fetch_one(transaction.as_mut())
			.await?
			.workshop_id
			.try_into()
			.map_err(|_| Error::bug())?
	};

	if map_update.check_steam {
		update_name_and_checksum(
			map_id,
			workshop_id,
			&state.http_client,
			&state.config,
			transaction.as_mut(),
		)
		.await?;
	}

	if !map_update.added_mappers.is_empty() {
		super::create::insert_mappers(
			MappersTable::Map(map_id),
			&map_update.added_mappers,
			transaction.as_mut(),
		)
		.await?;
	}

	if !map_update.removed_mappers.is_empty() {
		remove_mappers(
			MappersTable::Map(map_id),
			&map_update.removed_mappers,
			transaction.as_mut(),
		)
		.await?;
	}

	for course_update in &map_update.course_updates {
		update_course(map_id, course_update, &mut transaction).await?;
	}

	transaction.commit().await?;

	Ok(NoContent)
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
		return Err(Error::unknown_id("map", map_id));
	}

	if let Some(course_id) = map_update
		.course_updates
		.iter()
		.map(|course| course.id)
		.find(|course_id| !course_ids.contains(course_id))
	{
		return Err(Error::invalid("course").with_detail(json!({
			"id": course_id,
			"map_id": map_id
		})));
	}

	for course_update in &map_update.course_updates {
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
			.find(|filter| !filter_ids.iter().map(|row| row.id).contains(&filter.id))
		{
			return Err(Error::invalid("filter").with_detail(json!({
				"id": filter.id,
				"course_id": course_update.id
			})));
		}
	}

	Ok(())
}

async fn update_global_status(
	map_id: u16,
	global_status: GlobalStatus,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let result = sqlx::query! {
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

	if result.rows_affected() == 0 {
		return Err(Error::unknown_id("map", map_id));
	}

	audit!("updated global status for map", id = %map_id, %global_status);

	Ok(())
}

async fn update_description(
	map_id: u16,
	description: &str,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  description = ?
		WHERE
		  id = ?
		"#,
		description,
		map_id,
	}
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown_id("map", map_id));
	}

	audit!("updated map description", id = %map_id, %description);

	Ok(())
}

async fn update_workshop_id(
	map_id: u16,
	workshop_id: NonZeroU32,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  workshop_id = ?
		WHERE
		  id = ?
		"#,
		workshop_id.get(),
		map_id,
	}
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown_id("map", map_id));
	}

	audit!("updated workshop id", %map_id, %workshop_id);

	Ok(())
}

async fn update_name_and_checksum(
	map_id: u16,
	workshop_id: NonZeroU32,
	http_client: &reqwest::Client,
	config: &crate::Config,
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	let (workshop_map, checksum) = tokio::try_join! {
		workshop::Map::get(workshop_id, http_client),
		async { workshop::MapFile::download(workshop_id, config).await?.checksum().await },
	}?;

	let result = sqlx::query! {
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
	.execute(executor)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown_id("map", map_id));
	}

	trace! {
		id = %map_id,
		name = %workshop_map.name,
		%checksum,
		"updated workshop details for map",
	};

	Ok(())
}

async fn remove_mappers(
	table: MappersTable,
	mappers: &[SteamID],
	executor: impl MySqlExecutor<'_>,
) -> Result<()> {
	if mappers.is_empty() {
		return Ok(());
	}

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

async fn update_course(
	map_id: u16,
	update: &CourseUpdate,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	if !update.added_mappers.is_empty() {
		super::create::insert_mappers(
			MappersTable::Course(update.id),
			&update.added_mappers,
			transaction.as_mut(),
		)
		.await?;
	}

	if !update.removed_mappers.is_empty() {
		remove_mappers(
			MappersTable::Course(update.id),
			&update.removed_mappers,
			transaction.as_mut(),
		)
		.await?;
	}

	if let Some(description) = update.description.as_deref() {
		let result = sqlx::query! {
			r#"
			UPDATE
			  Courses
			SET
			  description = ?
			WHERE
			  id = ?
			"#,
			description,
			update.id,
		}
		.execute(transaction.as_mut())
		.await?;

		if result.rows_affected() == 0 {
			return Err(Error::invalid("course").with_detail(json!({
				"id": update.id,
				"map_id": map_id
			})));
		}

		audit!("updated course description", id = %update.id, %description);
	}

	for FilterUpdate { id, tier, ranked_status, notes } in update.filter_updates.iter() {
		if tier.is_none() && ranked_status.is_none() {
			continue;
		}

		if tier.is_some_and(|tier| tier > Tier::Death)
			&& matches!(ranked_status, Some(RankedStatus::Ranked))
		{
			return Err(Error::invalid("filter").with_detail(json!({
				"id": id,
				"reason": "tier too high for ranked status"
			})));
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

			delimiter = ",";
		}

		if let Some(notes) = notes.as_deref() {
			query.push(delimiter).push(" notes = ").push_bind(notes);
		}

		query.push(" WHERE id = ").push_bind(id);

		let result = query.build().execute(transaction.as_mut()).await?;

		if result.rows_affected() == 0 {
			return Err(Error::unknown_id("filter", id).with_detail(json!({
				"course_id": update.id
			})));
		}

		audit!("updated filter", %id, ?tier, ?ranked_status, ?notes);
	}

	Ok(())
}
