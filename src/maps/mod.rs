//! Everything related to KZ maps.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::collections::HashSet;
use std::iter;
use std::sync::Arc;

use axum::extract::FromRef;
use cs2kz::{GlobalStatus, SteamID};
use futures::TryFutureExt;
use query::UpdateQuery;
use sqlx::{MySql, Pool, QueryBuilder};

use crate::authentication::JwtState;
use crate::kz::MapIdentifier;
use crate::make_id::IntoID;
use crate::sqlx::{query, FilteredQuery, SqlErrorExt};
use crate::steam::workshop::{self, WorkshopID};
use crate::{Error, Result};

mod models;
pub use models::{
	Course,
	CourseID,
	CourseInfo,
	CourseUpdate,
	CreatedMap,
	FetchMapsRequest,
	Filter,
	FilterID,
	FilterUpdate,
	FullMap,
	MapID,
	MapInfo,
	MapUpdate,
	NewCourse,
	NewFilter,
	NewMap,
};

mod queries;
pub mod http;

/// A service for dealing with KZ maps as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct MapService
{
	database: Pool<MySql>,
	jwt_state: Arc<JwtState>,
	http_client: reqwest::Client,
	api_config: Arc<crate::Config>,
}

impl MapService
{
	/// Creates a new [`MapService`] instance.
	pub const fn new(
		database: Pool<MySql>,
		jwt_state: Arc<JwtState>,
		http_client: reqwest::Client,
		api_config: Arc<crate::Config>,
	) -> Self
	{
		Self { database, jwt_state, http_client, api_config }
	}

	/// Fetches a single map.
	pub async fn fetch_map(&self, map: MapIdentifier) -> Result<FullMap>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push(" WHERE ");

		match map {
			MapIdentifier::ID(id) => {
				query.push(" m.id = ").push_bind(id);
			}
			MapIdentifier::Name(name) => {
				query.push(" m.name LIKE ").push_bind(format!("%{name}%"));
			}
		}

		query.push(" ORDER BY m.id DESC ");

		let map = query
			.build_query_as::<FullMap>()
			.fetch_all(&self.database)
			.await?
			.into_iter()
			.reduce(FullMap::reduce)
			.ok_or_else(|| Error::not_found("map"))?;

		Ok(map)
	}

	/// Fetches many maps.
	pub async fn fetch_maps(&self, request: FetchMapsRequest) -> Result<(Vec<FullMap>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = FilteredQuery::new(queries::SELECT);

		if let Some(name) = request.name {
			query.filter(" m.name LIKE ", format!("%{name}%"));
		}

		if let Some(workshop_id) = request.workshop_id {
			query.filter(" m.workshop_id = ", workshop_id);
		}

		if let Some(global_status) = request.global_status {
			query.filter(" m.global_status = ", global_status);
		}

		if let Some(created_after) = request.created_after {
			query.filter(" m.created_on > ", created_after);
		}

		if let Some(created_before) = request.created_before {
			query.filter(" m.created_on < ", created_before);
		}

		// not entirely sure if this is correct?
		if let offset @ 1.. = *request.offset {
			query.filter(" m.id > ", offset);
		}

		query.push(" ORDER BY m.id DESC ");

		let maps = query
			.build_query_as::<FullMap>()
			.fetch_all(transaction.as_mut())
			.await
			.map(|maps| FullMap::flatten(maps, request.limit.into()))?;

		if maps.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((maps, total))
	}

	/// Submits a new map.
	pub async fn submit_map(&self, map: NewMap) -> Result<CreatedMap>
	{
		let (name, checksum) = tokio::try_join! {
			workshop::fetch_map_name(map.workshop_id, &self.http_client),
			workshop::MapFile::download(map.workshop_id, &self.api_config).and_then(|map_file| async move {
				map_file.checksum().await.map_err(|err| {
					Error::checksum(err).context(format!("workshop_id: {}", map.workshop_id))
				})
			}),
		}?;

		let mut transaction = self.database.begin().await?;

		let map_id = create_map(
			name,
			map.description,
			map.global_status,
			map.workshop_id,
			checksum,
			&mut transaction,
		)
		.await?;

		create_mappers(map_id, &map.mappers, &mut transaction).await?;
		create_courses(map_id, &map.courses, &mut transaction).await?;

		transaction.commit().await?;

		Ok(CreatedMap { map_id })
	}

	/// Updates an existing map.
	pub async fn update_map(&self, map_id: MapID, update: MapUpdate) -> Result<()>
	{
		let mut transaction = self.database.begin().await?;

		update_details(
			map_id,
			update.description,
			update.workshop_id,
			update.global_status,
			&mut transaction,
		)
		.await?;

		if update.check_steam || update.workshop_id.is_some() {
			update_name_and_checksum(
				map_id,
				update.workshop_id,
				&self.api_config,
				&self.http_client,
				&mut transaction,
			)
			.await?;
		}

		if let Some(added_mappers) = update.added_mappers {
			create_mappers(map_id, &added_mappers, &mut transaction).await?;
		}

		if let Some(removed_mappers) = update.removed_mappers {
			delete_mappers(map_id, &removed_mappers, &mut transaction).await?;
		}

		if let Some(course_updates) = update.course_updates {
			update_courses(map_id, course_updates, &mut transaction).await?;
		}

		transaction.commit().await?;

		tracing::info!(target: "cs2kz_api::audit_log", %map_id, "updated map");

		Ok(())
	}
}
/// Inserts a new map into the database and returns the generated [`MapID`].
async fn create_map(
	name: String,
	description: Option<String>,
	global_status: GlobalStatus,
	workshop_id: WorkshopID,
	checksum: u32,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<MapID>
{
	let deglobal_old_result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  global_status = -1
		WHERE
		  name = ?
		"#,
		name,
	}
	.execute(transaction.as_mut())
	.await?;

	match deglobal_old_result.rows_affected() {
		0 => {}
		1 => tracing::info! {
			target: "cs2kz_api::audit_log",
			%name,
			"degloballed old version of map",
		},
		amount => tracing::warn! {
			target: "cs2kz_api::audit_log",
			%name,
			%amount,
			"degloballed multiple versions of map",
		},
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
	.into_id::<MapID>()?;

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, "created map");

	Ok(map_id)
}

/// Inserts mappers into the database.
async fn create_mappers(
	map_id: MapID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
{
	let mut query = QueryBuilder::new("INSERT INTO Mappers (map_id, player_id)");

	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(map_id).push_bind(steam_id);
	});

	query
		.build()
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::not_found("mapper").context(err)
			} else {
				Error::from(err)
			}
		})?;

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, ?mappers, "created mappers");

	Ok(())
}

/// Inserts courses into the database and returns the generated [`CourseID`]s.
async fn create_courses(
	map_id: MapID,
	courses: &[NewCourse],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Vec<CourseID>>
{
	let mut query = QueryBuilder::new("INSERT INTO Courses (name, description, map_id)");

	query.push_values(courses, |mut query, course| {
		query
			.push_bind(course.name.as_deref())
			.push_bind(course.description.as_deref())
			.push_bind(map_id);
	});

	query.build().execute(transaction.as_mut()).await?;

	let course_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: CourseID`
		FROM
		  Courses
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		"#,
	}
	.fetch_all(transaction.as_mut())
	.await?;

	for (&course_id, course) in iter::zip(&course_ids, courses) {
		insert_course_mappers(course_id, &course.mappers, transaction).await?;
		insert_course_filters(course_id, &course.filters, transaction).await?;
	}

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, ?course_ids, "created courses");

	Ok(course_ids)
}

/// Inserts course mappers into the database.
async fn insert_course_mappers(
	course_id: CourseID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
{
	let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, player_id)");

	query.push_values(mappers, |mut query, steam_id| {
		query.push_bind(course_id).push_bind(steam_id);
	});

	query
		.build()
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::not_found("course mapper").context(err)
			} else {
				Error::from(err)
			}
		})?;

	tracing::debug!(target: "cs2kz_api::audit_log", %course_id, ?mappers, "created course mappers");

	Ok(())
}

/// Inserts course filters into the database and returns the generated
/// [`FilterID`]s.
async fn insert_course_filters(
	course_id: CourseID,
	filters: &[NewFilter; 4],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Vec<FilterID>>
{
	let mut query = QueryBuilder::new(
		r#"
		INSERT INTO
		  CourseFilters (
		    course_id,
		    mode_id,
		    teleports,
		    tier,
		    ranked_status,
		    notes
		  )
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

	query.build().execute(transaction.as_mut()).await?;

	let filter_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: FilterID`
		FROM
		  CourseFilters
		WHERE
		  id >= (
		    SELECT
		      LAST_INSERT_ID()
		  )
		"#,
	}
	.fetch_all(transaction.as_mut())
	.await?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?filter_ids,
		"created course filters",
	};

	Ok(filter_ids)
}

/// Updates only the metadata of a map (what's in the `Maps` table).
async fn update_details(
	map_id: MapID,
	description: Option<String>,
	workshop_id: Option<WorkshopID>,
	global_status: Option<GlobalStatus>,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
{
	if description.is_none() && workshop_id.is_none() && global_status.is_none() {
		return Ok(());
	}

	let mut query = UpdateQuery::new("Maps");

	if let Some(description) = description {
		query.set("description", description);
	}

	if let Some(workshop_id) = workshop_id {
		query.set("workshop_id", workshop_id);
	}

	if let Some(global_status) = global_status {
		query.set("global_status", global_status);
	}

	query.push(" WHERE id = ").push_bind(map_id);

	let query_result = query.build().execute(transaction.as_mut()).await?;

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("map")),
		n => assert_eq!(n, 1, "updated more than 1 map"),
	}

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, "updated map details");

	Ok(())
}

/// Updates a map's name and checksum by downloading it from the workshop.
async fn update_name_and_checksum(
	map_id: MapID,
	workshop_id: Option<WorkshopID>,
	api_config: &crate::Config,
	http_client: &reqwest::Client,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
{
	let workshop_id = if let Some(workshop_id) = workshop_id {
		workshop_id
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

	let (name, checksum) = tokio::try_join! {
		workshop::fetch_map_name(workshop_id, http_client),
		workshop::MapFile::download(workshop_id, api_config).and_then(|map| async move {
			map.checksum().await.map_err(|err| {
				Error::checksum(err).context(format!("map_id: {map_id}, workshop_id: {workshop_id}"))
			})
		}),
	}?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Maps
		SET
		  name = ?,
		  checksum = ?
		WHERE
		  id = ?
		"#,
		name,
		checksum,
		map_id,
	}
	.execute(transaction.as_mut())
	.await?;

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("map")),
		n => assert_eq!(n, 1, "updated more than 1 map"),
	}

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, "updated workshop details");

	Ok(())
}

/// Deletes mappers from the database.
async fn delete_mappers(
	map_id: MapID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
{
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
		  COUNT(map_id) count
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
		return Err(Error::must_have_mappers());
	}

	tracing::debug!(target: "cs2kz_api::audit_log", %map_id, ?mappers, "deleted mappers");

	Ok(())
}

/// Updates courses by applying [`CourseUpdate`]s and returns a list of
/// [`CourseID`]s of the courses that were actually updated.
async fn update_courses<C>(
	map_id: MapID,
	courses: C,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Vec<CourseID>>
where
	C: IntoIterator<Item = (CourseID, CourseUpdate)> + Send,
	C::IntoIter: Send,
{
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

	let courses = courses.into_iter().map(|(id, update)| {
		if valid_course_ids.remove(&id) {
			(id, Ok(update))
		} else {
			(id, Err(Error::mismatching_map_course(id, map_id)))
		}
	});

	let mut updated_course_ids = Vec::new();

	for (course_id, update) in courses {
		if let Some(course_id) = update_course(map_id, course_id, update?, transaction).await? {
			updated_course_ids.push(course_id);
		}
	}

	updated_course_ids.sort_unstable();

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%map_id,
		?updated_course_ids,
		"updated courses",
	};

	Ok(updated_course_ids)
}

/// Updates an individual course by applying a [`CourseUpdate`].
///
/// If the course was actually updated, `Some(course_id)` is returned, otherwise
/// `None`.
async fn update_course(
	map_id: MapID,
	course_id: CourseID,
	CourseUpdate {
		name,
		description,
		added_mappers,
		removed_mappers,
		filter_updates,
	}: CourseUpdate,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Option<CourseID>>
{
	if name.is_none()
		&& description.is_none()
		&& added_mappers.is_none()
		&& removed_mappers.is_none()
		&& filter_updates.is_none()
	{
		return Ok(None);
	}

	if name.is_some() || description.is_some() {
		let mut query = UpdateQuery::new("Courses");

		if let Some(name) = name {
			query.set("name", name);
		}

		if let Some(description) = description {
			query.set("description", description);
		}

		query.push(" WHERE id = ").push_bind(course_id);
		query.build().execute(transaction.as_mut()).await?;
	}

	if let Some(added_mappers) = added_mappers {
		insert_course_mappers(course_id, &added_mappers, transaction).await?;
	}

	if let Some(removed_mappers) = removed_mappers {
		delete_course_mappers(course_id, &removed_mappers, transaction).await?;
	}

	if let Some(filter_updates) = filter_updates {
		update_filters(map_id, course_id, filter_updates, transaction).await?;
	}

	Ok(Some(course_id))
}

/// Deletes course mappers from the database.
async fn delete_course_mappers(
	course_id: CourseID,
	mappers: &[SteamID],
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()>
{
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
		  COUNT(course_id) count
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
		return Err(Error::must_have_mappers());
	}

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%course_id,
		?mappers,
		"deleted course mappers",
	};

	Ok(())
}

/// Updates filters by applying [`FilterUpdate`]s and returns a list of
/// [`FilterID`]s of the filters that were actually updated.
async fn update_filters<F>(
	map_id: MapID,
	course_id: CourseID,
	filters: F,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Vec<FilterID>>
where
	F: IntoIterator<Item = (FilterID, FilterUpdate)> + Send,
	F::IntoIter: Send,
{
	let mut valid_filter_ids = sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: FilterID`
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
	.collect::<HashSet<_>>();

	let filters = filters.into_iter().map(|(id, update)| {
		if valid_filter_ids.remove(&id) {
			(id, Ok(update))
		} else {
			(id, Err(Error::mismatching_course_filter(id, course_id)))
		}
	});

	let mut updated_filter_ids = Vec::new();

	for (filter_id, update) in filters {
		if let Some(filter_id) = update_filter(filter_id, update?, transaction).await? {
			updated_filter_ids.push(filter_id);
		}
	}

	updated_filter_ids.sort_unstable();

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%map_id,
		%course_id,
		?updated_filter_ids,
		"updated filters",
	};

	Ok(updated_filter_ids)
}

/// Updates an individual filter by applying a [`FilterUpdate`].
///
/// If the filter was actually updated, `Some(filter_id)` is returned, otherwise
/// `None`.
async fn update_filter(
	filter_id: FilterID,
	FilterUpdate { tier, ranked_status, notes }: FilterUpdate,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<Option<FilterID>>
{
	if tier.is_none() && ranked_status.is_none() && notes.is_none() {
		return Ok(None);
	}

	let mut query = UpdateQuery::new("CourseFilters");

	if let Some(tier) = tier {
		query.set("tier", tier);
	}

	if let Some(ranked_status) = ranked_status {
		query.set("ranked_status", ranked_status);
	}

	if let Some(notes) = notes {
		query.set("notes", notes);
	}

	query.push(" WHERE id = ").push_bind(filter_id);
	query.build().execute(transaction.as_mut()).await?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		%filter_id,
		"updated course filter",
	};

	Ok(Some(filter_id))
}
