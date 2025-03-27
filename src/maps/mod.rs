mod course;
mod description;
mod filter;
mod id;
mod name;
mod state;
mod stream;
mod tier;

use std::{
	collections::{HashMap, btree_map::BTreeMap},
	fmt,
	pin::pin,
};

use futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt};
use serde::Serialize;
use sqlx::Row;
use tracing::Instrument;
use utoipa::ToSchema;

pub use self::{
	course::{
		CourseDescription,
		CourseId,
		CourseLocalId,
		CourseName,
		InvalidCourseDescription,
		InvalidCourseName,
		ParseCourseIdError,
		ParseCourseLocalIdError,
	},
	description::{InvalidMapDescription, MapDescription},
	filter::{FilterId, FilterNotes, InvalidFilterNotes, ParseFilterIdError},
	id::{MapId, ParseMapIdError},
	name::{InvalidMapName, MapName},
	state::MapState,
	tier::Tier,
};
use crate::{
	checksum::Checksum,
	database::{DatabaseConnection, DatabaseError, DatabaseResult},
	event_queue::{self, Event},
	mode::Mode,
	steam::{self, workshop::WorkshopId},
	stream::StreamExt as _,
	time::Timestamp,
	users::{InvalidUsername, UserId, Username},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Map
{
	pub id: MapId,
	pub workshop_id: WorkshopId,
	pub name: MapName,
	pub description: Option<MapDescription>,
	pub state: MapState,

	/// A checksum of the map's `.vpk` file
	pub checksum: Checksum,

	#[serde(serialize_with = "crate::serde::ser::map_as_set")]
	pub courses: BTreeMap<CourseId, Course>,
	pub created_by: Mapper,
	pub created_at: Timestamp,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Mapper
{
	pub id: UserId,
	pub name: Username,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Course
{
	/// The course's ID chosen by the API
	///
	/// This is unique across all courses registered by the API.
	pub id: CourseId,

	/// The course's ID chosen by the mapper
	///
	/// This is unique across all courses within a map.
	pub local_id: CourseLocalId,
	pub name: CourseName,
	pub description: Option<CourseDescription>,

	#[serde(serialize_with = "crate::serde::ser::map_as_set")]
	pub mappers: BTreeMap<UserId, Mapper>,

	pub filters: Filters,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Filters
{
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cs2: Option<CS2Filters>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub csgo: Option<CSGOFilters>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CS2Filters
{
	pub vnl: Filter,
	pub ckz: Filter,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CSGOFilters
{
	pub kzt: Filter,
	pub skz: Filter,
	pub vnl: Filter,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Filter
{
	pub id: FilterId,
	pub nub_tier: Tier,
	pub pro_tier: Tier,
	pub ranked: bool,
	pub notes: Option<FilterNotes>,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: Option<&str>,
	#[builder(default = MapState::Approved)] state: MapState,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(*)
		 FROM Maps
		 WHERE state = ?
		 AND name LIKE COALESCE(?, name)",
		state,
		name.map(|name| format!("%{name}%")),
	)
	.fetch_one(conn.as_raw())
	.map_err(DatabaseError::from)
	.and_then(async |row| row.try_into().map_err(DatabaseError::convert_count))
	.await
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: Option<&str>,
	#[builder(default = MapState::Approved)] state: MapState,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Map>>
{
	let row_stream = sqlx::query!(
		"WITH RelevantMaps AS (
		   SELECT *, MATCH (name) AGAINST (?) AS name_score
		   FROM Maps
		   WHERE name LIKE COALESCE(?, name)
		   AND state = ?
		   ORDER BY name_score DESC, id DESC
		   LIMIT ?, ?
		 )
		 SELECT
		   m.id AS `id: MapId`,
		   m.workshop_id AS `workshop_id: WorkshopId`,
		   m.name AS `name: MapName`,
		   m.description AS `description: MapDescription`,
		   m.state AS `state: MapState`,
		   m.checksum AS `checksum: Checksum`,
		   m.created_at AS `created_at: Timestamp`,
		   ma.id AS `mapper_id: UserId`,
		   ma.name AS `mapper_name: Username`,
		   c.id AS `course_id: CourseId`,
		   c.local_id AS `course_local_id: CourseLocalId`,
		   c.name AS `course_name: CourseName`,
		   c.description AS `course_description: CourseDescription`,
		   cma.id AS `course_mapper_id: UserId`,
		   cma.name AS `course_mapper_name: Username`,
		   f.id AS `filter_id: FilterId`,
		   f.mode AS `filter_mode: Mode`,
		   f.nub_tier AS `filter_nub_tier: Tier`,
		   f.pro_tier AS `filter_pro_tier: Tier`,
		   f.ranked AS `filter_ranked: bool`,
		   f.notes AS `filter_notes: FilterNotes`
		 FROM RelevantMaps AS m
		 INNER JOIN Users AS ma ON ma.id = m.created_by
		 INNER JOIN Courses AS c ON c.map_id = m.id
		 INNER JOIN CourseMappers ON CourseMappers.course_id = c.id
		 INNER JOIN Users AS cma ON cma.id = CourseMappers.user_id
		 INNER JOIN Filters AS f ON f.course_id = c.id
		 ORDER BY m.name_score DESC, m.id DESC, ma.id ASC, c.id ASC, cma.id ASC, f.mode ASC",
		name,
		name.map(|name| format!("%{name}%")),
		state,
		offset,
		limit,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current());

	self::stream::from_raw!(row_stream)
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Map>>
{
	let row_stream = sqlx::query!(
		"SELECT
		   m.id AS `id: MapId`,
		   m.workshop_id AS `workshop_id: WorkshopId`,
		   m.name AS `name: MapName`,
		   m.description AS `description: MapDescription`,
		   m.state AS `state: MapState`,
		   m.checksum AS `checksum: Checksum`,
		   m.created_at AS `created_at: Timestamp`,
		   ma.id AS `mapper_id: UserId`,
		   ma.name AS `mapper_name: Username`,
		   c.id AS `course_id: CourseId`,
		   c.local_id AS `course_local_id: CourseLocalId`,
		   c.name AS `course_name: CourseName`,
		   c.description AS `course_description: CourseDescription`,
		   cma.id AS `course_mapper_id: UserId`,
		   cma.name AS `course_mapper_name: Username`,
		   f.id AS `filter_id: FilterId`,
		   f.mode AS `filter_mode: Mode`,
		   f.nub_tier AS `filter_nub_tier: Tier`,
		   f.pro_tier AS `filter_pro_tier: Tier`,
		   f.ranked AS `filter_ranked: bool`,
		   f.notes AS `filter_notes: FilterNotes`
		 FROM Maps AS m
		 INNER JOIN Users AS ma ON ma.id = m.created_by
		 INNER JOIN Courses AS c ON c.map_id = m.id
		 INNER JOIN CourseMappers ON CourseMappers.course_id = c.id
		 INNER JOIN Users AS cma ON cma.id = CourseMappers.user_id
		 INNER JOIN Filters AS f ON f.course_id = c.id
		 WHERE m.id = ?
		 AND m.state != -1
		 ORDER BY m.id DESC, ma.id ASC, c.id ASC, cma.id ASC, f.mode ASC",
		map_id,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current());

	pin!(self::stream::from_raw!(row_stream)).try_next().await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_name(
	#[builder(start_fn)] map_name: &str,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Map>>
{
	let row_stream = sqlx::query!(
		"SELECT
		   m.id AS `id: MapId`,
		   m.workshop_id AS `workshop_id: WorkshopId`,
		   m.name AS `name: MapName`,
		   m.description AS `description: MapDescription`,
		   m.state AS `state: MapState`,
		   m.checksum AS `checksum: Checksum`,
		   m.created_at AS `created_at: Timestamp`,
		   ma.id AS `mapper_id: UserId`,
		   ma.name AS `mapper_name: Username`,
		   c.id AS `course_id: CourseId`,
		   c.local_id AS `course_local_id: CourseLocalId`,
		   c.name AS `course_name: CourseName`,
		   c.description AS `course_description: CourseDescription`,
		   cma.id AS `course_mapper_id: UserId`,
		   cma.name AS `course_mapper_name: Username`,
		   f.id AS `filter_id: FilterId`,
		   f.mode AS `filter_mode: Mode`,
		   f.nub_tier AS `filter_nub_tier: Tier`,
		   f.pro_tier AS `filter_pro_tier: Tier`,
		   f.ranked AS `filter_ranked: bool`,
		   f.notes AS `filter_notes: FilterNotes`
		 FROM Maps AS m
		 INNER JOIN Users AS ma ON ma.id = m.created_by
		 INNER JOIN Courses AS c ON c.map_id = m.id
		 INNER JOIN CourseMappers ON CourseMappers.course_id = c.id
		 INNER JOIN Users AS cma ON cma.id = CourseMappers.user_id
		 INNER JOIN Filters AS f ON f.course_id = c.id
		 WHERE m.name = ?
		 AND m.state != -1
		 ORDER BY m.id DESC, ma.id ASC, c.id ASC, cma.id ASC, f.mode ASC",
		map_name,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current());

	pin!(self::stream::from_raw!(row_stream)).try_next().await
}

#[derive(Debug)]
pub struct Metadata
{
	pub id: MapId,
	pub state: MapState,
	pub created_by: UserId,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_metadata(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Metadata>>
{
	sqlx::query_as!(
		Metadata,
		"SELECT
		   id AS `id: MapId`,
		   state AS `state: MapState`,
		   created_by AS `created_by: UserId`
		 FROM Maps
		 WHERE id = ?",
		map_id,
	)
	.fetch_optional(conn.as_raw())
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get_filters(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	#[builder(default = MapState::Approved)] map_state: MapState,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Filter>>
{
	sqlx::query_as!(
		Filter,
		"WITH RelevantMaps AS (
		   SELECT *
		   FROM Maps
		   WHERE state = ?
		   ORDER BY id DESC
		   LIMIT ?, ?
		 )
		 SELECT
		   f.id AS `id: FilterId`,
		   f.nub_tier AS `nub_tier: Tier`,
		   f.pro_tier AS `pro_tier: Tier`,
		   f.ranked AS `ranked: bool`,
		   f.notes AS `notes: FilterNotes`
		 FROM Filters AS f
		 INNER JOIN Courses AS c ON c.id = f.course_id
		 INNER JOIN RelevantMaps AS m ON m.id = c.map_id
		 ORDER BY f.id DESC",
		map_state,
		offset,
		limit,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub async fn get_mode_by_filter_id(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Mode>>
{
	sqlx::query_scalar!(
		"SELECT mode AS `mode: Mode`
		 FROM Filters
		 WHERE id = ?",
		filter_id,
	)
	.fetch_optional(conn.as_raw())
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_filter_id(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(start_fn)] mode: Mode,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<FilterId>>
{
	sqlx::query_scalar!(
		"SELECT id AS `id: FilterId`
		 FROM Filters
		 WHERE course_id = ? AND mode = ?",
		course_id,
		mode,
	)
	.fetch_optional(conn.as_raw())
	.map_err(DatabaseError::from)
	.await
}

#[derive(Debug, Display, Error, From)]
pub enum CreateMapError
{
	#[display("invalid mapper ID '{_0}'")]
	#[error(ignore)]
	InvalidMapperId(UserId),

	#[display("mapper with ID '{id}' has an invalid name: {error}")]
	InvalidMapperName
	{
		id: UserId,

		#[error(source)]
		error: InvalidUsername,
	},

	#[from]
	SteamApiError(steam::ApiError),

	#[display("map is frozen")]
	MapIsFrozen
	{
		id: MapId, state: MapState
	},

	#[display("mapper did not create previous versions of the map")]
	NotTheMapper,

	#[from(DatabaseError, sqlx::Error)]
	Database(DatabaseError),
}

#[derive(Debug, Builder)]
pub struct NewCourse<NewMappers: IntoIterator<Item = UserId>>
{
	#[builder(start_fn)]
	local_id: CourseLocalId,
	name: CourseName,
	description: Option<CourseDescription>,
	mappers: NewMappers,
	filters: NewFilters,
}

#[derive(Debug, Builder)]
pub struct NewFilters
{
	cs2: Option<NewCS2Filters>,
	csgo: Option<NewCSGOFilters>,
}

#[derive(Debug, Builder)]
pub struct NewCS2Filters
{
	vnl: NewFilter,
	ckz: NewFilter,
}

#[derive(Debug, Builder)]
pub struct NewCSGOFilters
{
	kzt: NewFilter,
	skz: NewFilter,
	vnl: NewFilter,
}

#[derive(Debug, Builder)]
pub struct NewFilter
{
	nub_tier: Tier,
	pro_tier: Tier,
	ranked: bool,
	notes: Option<FilterNotes>,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create<I, CourseMappers>(
	#[builder(start_fn)] workshop_id: WorkshopId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: MapName,
	description: Option<MapDescription>,
	#[builder(default = MapState::WIP)] state: MapState,
	checksum: Checksum,
	created_by: UserId,
	courses: I,
) -> Result<MapId, CreateMapError>
where
	I: IntoIterator<Item = NewCourse<CourseMappers>> + fmt::Debug,
	CourseMappers: IntoIterator<Item = UserId> + fmt::Debug,
{
	match self::invalidate(&name)
		.check_created_by(created_by)
		.exec(&mut *conn)
		.await?
	{
		0 => { /* first submission */ },
		1 => tracing::debug!("invalidated old version"),
		n => tracing::warn!("invalidated {n} old versions"),
	}

	let map_id = sqlx::query!(
		"INSERT INTO Maps (workshop_id, name, description, state, checksum, created_by)
		 VALUES (?, ?, ?, ?, ?, ?)
		 RETURNING id",
		workshop_id,
		name,
		description,
		state,
		checksum,
		created_by,
	)
	.fetch_one(conn.as_raw())
	.and_then(async |row| row.try_get(0))
	.await?;

	async {
		for course in courses {
			let course_id = create_course(map_id).course(course).exec(&mut *conn).await?;

			tracing::debug!(id = %course_id, "created course");
		}

		Result::<(), CreateMapError>::Ok(())
	}
	.instrument(tracing::debug_span!("create_courses", %map_id))
	.await?;

	event_queue::dispatch(Event::MapCreated { id: map_id, name });

	Ok(map_id)
}

#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn create_course<CourseMappers>(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	course: NewCourse<CourseMappers>,
) -> Result<CourseId, CreateMapError>
where
	CourseMappers: IntoIterator<Item = UserId> + fmt::Debug,
{
	let course_id = sqlx::query!(
		"INSERT INTO Courses (map_id, local_id, name, description)
		 VALUES (?, ?, ?, ?)
		 RETURNING id",
		map_id,
		course.local_id,
		course.name,
		course.description,
	)
	.fetch_one(conn.as_raw())
	.and_then(async |row| row.try_get(0))
	.await?;

	for user_id in course.mappers {
		create_course_mapper(course_id).user_id(user_id).exec(&mut *conn).await?;
		tracing::debug!(id = %user_id, "created course mapper");
	}

	async {
		if let Some(NewCSGOFilters { kzt, skz, vnl }) = course.filters.csgo {
			for (mode, filter) in [
				(Mode::KZTimer, kzt),
				(Mode::SimpleKZ, skz),
				(Mode::VanillaCSGO, vnl),
			] {
				let filter_id = create_filter(course_id, mode)
					.nub_tier(filter.nub_tier)
					.pro_tier(filter.pro_tier)
					.ranked(filter.ranked)
					.maybe_notes(filter.notes)
					.exec(&mut *conn)
					.await?;

				tracing::debug!(?mode, id = %filter_id, "created filter");
			}

			tracing::debug!("created csgo filters");
		}

		if let Some(NewCS2Filters { vnl, ckz }) = course.filters.cs2 {
			for (mode, filter) in [(Mode::Vanilla, vnl), (Mode::Classic, ckz)] {
				let filter_id = create_filter(course_id, mode)
					.nub_tier(filter.nub_tier)
					.pro_tier(filter.pro_tier)
					.ranked(filter.ranked)
					.maybe_notes(filter.notes)
					.exec(&mut *conn)
					.await?;

				tracing::debug!(?mode, id = %filter_id, "created filter");
			}

			tracing::debug!("created cs2 filters");
		}

		DatabaseResult::Ok(())
	}
	.instrument(tracing::debug_span!("create_filters", %course_id))
	.await?;

	Ok(course_id)
}

#[tracing::instrument(level = "debug", skip(conn), err)]
#[builder(finish_fn = exec)]
async fn create_course_mapper(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	user_id: UserId,
) -> DatabaseResult<()>
{
	sqlx::query!(
		"INSERT INTO CourseMappers (course_id, user_id)
		 VALUES (?, ?)",
		course_id,
		user_id
	)
	.execute(conn.as_raw())
	.await?;

	Ok(())
}

#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn create_filter(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(start_fn)] mode: Mode,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	nub_tier: Tier,
	pro_tier: Tier,
	ranked: bool,
	notes: Option<FilterNotes>,
) -> DatabaseResult<FilterId>
{
	let filter_id = sqlx::query!(
		"INSERT INTO Filters (course_id, mode, nub_tier, pro_tier, ranked, notes)
		 VALUES (?, ?, ?, ?, ?, ?)
		 RETURNING id",
		course_id,
		mode,
		nub_tier,
		pro_tier,
		ranked,
		notes,
	)
	.fetch_one(conn.as_raw())
	.and_then(async |row| row.try_get(0))
	.await?;

	Ok(filter_id)
}

#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn invalidate(
	#[builder(start_fn)] name: &MapName,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	check_created_by: UserId,
) -> Result<u64, CreateMapError>
{
	let old_versions = sqlx::query!(
		"SELECT
		   id AS `id: MapId`,
		   state AS `state: MapState`,
		   created_by AS `created_by: UserId`
		 FROM Maps
		 WHERE name = ?",
		name,
	)
	.fetch(conn.as_raw())
	.map_ok(|row| (row.id, row.state, row.created_by))
	.try_collect::<Vec<_>>()
	.await?;

	if let Some(&(id, state, ..)) =
		old_versions.iter().find(|&&(_, ref state, _)| state.is_frozen())
	{
		return Err(CreateMapError::MapIsFrozen { id, state });
	}

	let Some(&(.., created_by)) = old_versions.first() else {
		return Ok(0);
	};

	if created_by != check_created_by {
		return Err(CreateMapError::NotTheMapper);
	}

	let (conn, query) = conn.as_parts();

	query.reset();
	query.push("UPDATE Maps SET state = -1 WHERE id IN");
	query.push_tuples(old_versions, |mut query, (id, ..)| {
		query.push_bind(id);
	});

	query
		.build()
		.execute(conn)
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(CreateMapError::from)
		.await
}

#[derive(Debug, Builder)]
pub struct CourseUpdate<
	AddedMappers: IntoIterator<Item = UserId>,
	RemovedMappers: IntoIterator<Item = UserId>,
	FilterUpdates: IntoIterator<Item = (Mode, FilterUpdate)>,
> {
	name: Option<CourseName>,
	description: Option<CourseDescription>,
	added_mappers: AddedMappers,
	removed_mappers: RemovedMappers,
	filter_updates: FilterUpdates,
}

#[derive(Debug, Builder)]
pub struct FilterUpdate
{
	nub_tier: Option<Tier>,
	pro_tier: Option<Tier>,
	ranked: Option<bool>,
	notes: Option<FilterNotes>,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to update map: {_variant}")]
pub enum UpdateMapError
{
	#[display("invalid map ID")]
	InvalidMapId,

	#[display("invalid course ID")]
	#[error(ignore)]
	InvalidCourseLocalId(CourseLocalId),

	#[display("{_0}")]
	#[from(forward)]
	DatabaseError(DatabaseError),
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn update<I, AddedMappers, RemovedMappers, FilterUpdates>(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	workshop_id: WorkshopId,
	name: MapName,
	description: Option<MapDescription>,
	checksum: Checksum,
	course_updates: I,
) -> Result<(), UpdateMapError>
where
	I: IntoIterator<
			Item = (CourseLocalId, CourseUpdate<AddedMappers, RemovedMappers, FilterUpdates>),
		> + fmt::Debug,
	AddedMappers: IntoIterator<Item = UserId> + fmt::Debug,
	RemovedMappers: IntoIterator<Item = UserId> + fmt::Debug,
	FilterUpdates: IntoIterator<Item = (Mode, FilterUpdate)> + fmt::Debug,
{
	let updated = sqlx::query!(
		"UPDATE Maps
		 SET workshop_id = ?,
		     name = ?,
			 description = COALESCE(?, description),
			 checksum = ?
		 WHERE id = ?",
		workshop_id,
		name,
		description,
		checksum,
		map_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await?;

	tracing::debug!(%map_id, "updated map");

	if !updated {
		return Err(UpdateMapError::InvalidMapId);
	}

	let course_ids = sqlx::query!(
		"SELECT
		   id AS `id: CourseId`,
		   local_id AS `local_id: CourseLocalId`
		 FROM Courses
		 WHERE map_id = ?",
		map_id,
	)
	.fetch(conn.as_raw())
	.map_ok(|row| (row.local_id, row.id))
	.try_collect::<HashMap<CourseLocalId, CourseId>>()
	.await?;

	for (local_id, course_update) in course_updates {
		let &course_id = course_ids
			.get(&local_id)
			.ok_or(UpdateMapError::InvalidCourseLocalId(local_id))?;

		update_course(course_id)
			.maybe_name(course_update.name)
			.maybe_description(course_update.description)
			.added_mappers(course_update.added_mappers)
			.removed_mappers(course_update.removed_mappers)
			.filter_updates(course_update.filter_updates)
			.exec(&mut *conn)
			.await?;

		tracing::debug!(%local_id, "updated course");
	}

	Ok(())
}

#[tracing::instrument(level = "debug", skip(conn), err)]
#[builder(finish_fn = exec)]
async fn update_course<AddedMappers, RemovedMappers, FilterUpdates>(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: Option<CourseName>,
	description: Option<CourseDescription>,
	added_mappers: AddedMappers,
	removed_mappers: RemovedMappers,
	filter_updates: FilterUpdates,
) -> DatabaseResult<()>
where
	AddedMappers: IntoIterator<Item = UserId> + fmt::Debug,
	RemovedMappers: IntoIterator<Item = UserId> + fmt::Debug,
	FilterUpdates: IntoIterator<Item = (Mode, FilterUpdate)> + fmt::Debug,
{
	let (conn, query) = conn.as_parts();

	sqlx::query!(
		"UPDATE Courses
		 SET name = COALESCE(?, name),
		     description = COALESCE(?, description)
		 WHERE id = ?",
		name,
		description,
		course_id,
	)
	.execute(&mut *conn)
	.await?;

	{
		query.reset();
		query.push("INSERT INTO CourseMappers (course_id, user_id)");

		let mut had_values = false;

		query.push_values(added_mappers, |mut query, user_id| {
			query.push_bind(user_id);
			had_values = true;
		});

		if had_values {
			query.build().execute(&mut *conn).await?;
		}
	}

	{
		query.reset();
		query.push("DELETE FROM CourseMappers WHERE course_id = ");
		query.push_bind(course_id);
		query.push(" AND user_id IN ");

		let mut had_values = false;

		query.push_tuples(removed_mappers, |mut query, user_id| {
			query.push_bind(user_id);
			had_values = true;
		});

		if had_values {
			query.build().execute(&mut *conn).await?;
		}
	}

	for (mode, filter_update) in filter_updates {
		sqlx::query!(
			"UPDATE Filters
			 SET nub_tier = COALESCE(?, nub_tier),
			     pro_tier = COALESCE(?, pro_tier),
				 ranked = COALESCE(?, ranked),
				 notes = COALESCE(?, notes)
			 WHERE course_id = ? AND mode = ?",
			filter_update.nub_tier,
			filter_update.pro_tier,
			filter_update.ranked,
			filter_update.notes,
			course_id,
			mode,
		)
		.execute(&mut *conn)
		.await?;

		tracing::debug!(?mode, "updated filter");
	}

	Ok(())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn set_state(
	#[builder(start_fn)] map_id: MapId,
	#[builder(start_fn)] state: MapState,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	let updated = sqlx::query!(
		"UPDATE Maps
		 SET state = ?
		 WHERE id = ?",
		state,
		map_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.await?;

	if updated {
		event_queue::dispatch(Event::MapApproved { id: map_id });
	}

	Ok(updated)
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	created_by: Option<UserId>,
) -> DatabaseResult<u64>
{
	sqlx::query!(
		"DELETE FROM Maps
		 WHERE created_by = COALESCE(?, created_by)
		 LIMIT ?",
		created_by,
		count,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected())
	.map_err(DatabaseError::from)
	.await
}
