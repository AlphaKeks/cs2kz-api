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
use {
	crate::{
		checksum::Checksum,
		database::{self, DatabaseError, DatabaseResult, QueryBuilder},
		error::ResultExt,
		event_queue::{self, Event},
		game::Game,
		mode::Mode,
		steam::{
			self,
			workshop::{self, WorkshopId},
		},
		stream::StreamExt as _,
		time::Timestamp,
		users::{InvalidUsername, UserId, Username},
	},
	futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt},
	serde::Serialize,
	sqlx::Row,
	std::{
		collections::{HashMap, btree_map::BTreeMap},
		fmt,
		fs::{self, File},
		io,
		path::Path,
		pin::pin,
	},
	tokio::task,
	tracing::Instrument,
	utoipa::ToSchema,
};

mod course;
mod description;
mod filter;
mod id;
mod name;
mod state;
mod stream;
mod tier;

#[derive(Debug, Serialize, ToSchema)]
pub struct Map
{
	pub id: MapId,
	pub workshop_id: WorkshopId,
	pub name: MapName,
	pub description: MapDescription,
	pub game: Game,
	pub state: MapState,

	/// A checksum of the map's `.vpk` file
	pub checksum: Checksum,

	#[serde(serialize_with = "crate::serde::ser::map_values")]
	#[schema(value_type = [Course])]
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
	pub description: CourseDescription,

	#[serde(serialize_with = "crate::serde::ser::map_values")]
	#[schema(value_type = [Mapper])]
	pub mappers: BTreeMap<UserId, Mapper>,

	pub filters: Filters,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum Filters
{
	CS2
	{
		vnl: Filter, ckz: Filter
	},

	CSGO
	{
		kzt: Filter, skz: Filter, vnl: Filter
	},
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Filter
{
	pub id: FilterId,
	pub nub_tier: Tier,
	pub pro_tier: Tier,
	pub ranked: bool,
	pub notes: FilterNotes,
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: Option<&str>,
	state: Option<MapState>,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(*)
		 FROM Maps
		 WHERE game = ?
		 AND state = COALESCE(?, state)
		 AND name LIKE COALESCE(?, name)",
		game,
		state,
		name.map(|name| format!("%{name}%")),
	)
	.fetch_one(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.and_then(async |row| row.try_into().map_err(DatabaseError::convert_count))
	.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get(
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: Option<&str>,
	state: Option<MapState>,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Map>>
{
	let row_stream = sqlx::query!(
		"WITH RelevantMaps AS (
		   SELECT *, MATCH (name) AGAINST (?) AS name_score
		   FROM Maps
		   WHERE game = ?
		   AND state = COALESCE(?, state)
		   AND name LIKE COALESCE(?, name)
		   ORDER BY name_score DESC, id DESC
		   LIMIT ?, ?
		 )
		 SELECT
		   m.id AS `id: MapId`,
		   m.workshop_id AS `workshop_id: WorkshopId`,
		   m.name AS `name: MapName`,
		   m.description AS `description: MapDescription`,
		   m.game AS `game: Game`,
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
		 INNER JOIN Filters AS f ON f.course_id = c.id AND f.mode IN (?, ?, ?)
		 ORDER BY m.name_score DESC, m.id DESC, c.id ASC, f.mode ASC, ma.id ASC, cma.id ASC",
		name,
		game,
		state,
		name.map(|name| format!("%{name}%")),
		offset,
		limit,
		match game {
			Game::CS2 => Some(Mode::VanillaCS2),
			Game::CSGO => Some(Mode::KZTimer),
		},
		match game {
			Game::CS2 => Some(Mode::Classic),
			Game::CSGO => Some(Mode::SimpleKZ),
		},
		match game {
			Game::CS2 => None,
			Game::CSGO => Some(Mode::VanillaCSGO),
		},
	)
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current());

	self::stream::from_raw!(row_stream)
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<Map>>
{
	let row_stream = sqlx::query!(
		"SELECT
		   m.id AS `id: MapId`,
		   m.workshop_id AS `workshop_id: WorkshopId`,
		   m.name AS `name: MapName`,
		   m.description AS `description: MapDescription`,
		   m.game AS `game: Game`,
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
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current());

	pin!(self::stream::from_raw!(row_stream)).try_next().await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_name(
	#[builder(start_fn)] map_name: &str,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<Map>>
{
	let row_stream = sqlx::query!(
		"SELECT
		   m.id AS `id: MapId`,
		   m.workshop_id AS `workshop_id: WorkshopId`,
		   m.name AS `name: MapName`,
		   m.description AS `description: MapDescription`,
		   m.game AS `game: Game`,
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
	.fetch(db_conn.raw_mut())
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

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_metadata(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
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
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get_filters(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
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
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub async fn get_mode_by_filter_id(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<Mode>>
{
	sqlx::query_scalar!(
		"SELECT mode AS `mode: Mode`
		 FROM Filters
		 WHERE id = ?",
		filter_id,
	)
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_filter_id(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(start_fn)] mode: Mode,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<FilterId>>
{
	sqlx::query_scalar!(
		"SELECT id AS `id: FilterId`
		 FROM Filters
		 WHERE course_id = ? AND mode = ?",
		course_id,
		mode,
	)
	.fetch_optional(db_conn.raw_mut())
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
	DatabaseError(DatabaseError),
}

#[derive(Debug, Builder)]
pub struct NewCourse<NewMappers: IntoIterator<Item = UserId>>
{
	#[builder(start_fn)]
	local_id: CourseLocalId,
	name: CourseName,
	#[builder(default)]
	description: CourseDescription,
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

	#[builder(default)]
	notes: FilterNotes,
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create<I, CourseMappers>(
	#[builder(start_fn)] workshop_id: WorkshopId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: MapName,
	#[builder(default)] description: MapDescription,
	game: Game,
	#[builder(default = MapState::WIP)] state: MapState,
	checksum: Checksum,
	created_by: UserId,
	courses: I,
) -> Result<MapId, CreateMapError>
where
	I: IntoIterator<Item = NewCourse<CourseMappers>> + fmt::Debug,
	CourseMappers: IntoIterator<Item = UserId> + fmt::Debug,
{
	match self::invalidate(&name, game)
		.check_created_by(created_by)
		.exec(&mut *db_conn)
		.await?
	{
		0 => { /* first submission */ },
		1 => debug!("invalidated old version"),
		n => warn!("invalidated {n} old versions"),
	}

	let map_id = sqlx::query!(
		"INSERT INTO Maps (workshop_id, name, description, game, state, checksum, created_by)
		 VALUES (?, ?, ?, ?, ?, ?, ?)
		 RETURNING id",
		workshop_id,
		name,
		description,
		game,
		state,
		checksum,
		created_by,
	)
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get(0))
	.await?;

	async {
		for course in courses {
			let course_id = create_course(map_id).course(course).exec(&mut *db_conn).await?;

			debug!(id = %course_id, "created course");
		}

		Result::<(), CreateMapError>::Ok(())
	}
	.instrument(tracing::debug_span!("create_courses", %map_id))
	.await?;

	event_queue::dispatch(Event::MapCreated { id: map_id, name });

	Ok(map_id)
}

#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn create_course<CourseMappers>(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
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
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get(0))
	.await?;

	for user_id in course.mappers {
		create_course_mapper(course_id)
			.user_id(user_id)
			.exec(&mut *db_conn)
			.await?;
		debug!(id = %user_id, "created course mapper");
	}

	async {
		if let Some(NewCSGOFilters { kzt, skz, vnl }) = course.filters.csgo {
			for (mode, filter) in
				[(Mode::KZTimer, kzt), (Mode::SimpleKZ, skz), (Mode::VanillaCSGO, vnl)]
			{
				let filter_id = create_filter(course_id, mode)
					.nub_tier(filter.nub_tier)
					.pro_tier(filter.pro_tier)
					.ranked(filter.ranked)
					.notes(filter.notes)
					.exec(&mut *db_conn)
					.await?;

				debug!(?mode, id = %filter_id, "created filter");
			}

			debug!("created csgo filters");
		}

		if let Some(NewCS2Filters { vnl, ckz }) = course.filters.cs2 {
			for (mode, filter) in [(Mode::VanillaCS2, vnl), (Mode::Classic, ckz)] {
				let filter_id = create_filter(course_id, mode)
					.nub_tier(filter.nub_tier)
					.pro_tier(filter.pro_tier)
					.ranked(filter.ranked)
					.notes(filter.notes)
					.exec(&mut *db_conn)
					.await?;

				debug!(?mode, id = %filter_id, "created filter");
			}

			debug!("created cs2 filters");
		}

		DatabaseResult::Ok(())
	}
	.instrument(tracing::debug_span!("create_filters", %course_id))
	.await?;

	Ok(course_id)
}

#[instrument(level = "debug", skip(db_conn), err)]
#[builder(finish_fn = exec)]
async fn create_course_mapper(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	user_id: UserId,
) -> DatabaseResult<()>
{
	sqlx::query!(
		"INSERT INTO CourseMappers (course_id, user_id)
		 VALUES (?, ?)",
		course_id,
		user_id
	)
	.execute(db_conn.raw_mut())
	.await?;

	Ok(())
}

#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn create_filter(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(start_fn)] mode: Mode,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	nub_tier: Tier,
	pro_tier: Tier,
	ranked: bool,
	#[builder(default)] notes: FilterNotes,
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
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get(0))
	.await?;

	Ok(filter_id)
}

#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn invalidate(
	#[builder(start_fn)] name: &MapName,
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	check_created_by: UserId,
) -> Result<u64, CreateMapError>
{
	let old_versions = sqlx::query!(
		"SELECT
		   id AS `id: MapId`,
		   state AS `state: MapState`,
		   created_by AS `created_by: UserId`
		 FROM Maps
		 WHERE name = ?
		 AND game = ?",
		name,
		game,
	)
	.fetch(db_conn.raw_mut())
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

	let mut query = QueryBuilder::new("UPDATE Maps SET state = -1 WHERE id IN");

	query.push_tuples(old_versions, |mut query, (id, ..)| {
		query.push_bind(id);
	});

	query
		.build()
		.execute(db_conn.raw_mut())
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
	#[builder(required)]
	name: Option<CourseName>,
	#[builder(required)]
	description: Option<CourseDescription>,
	added_mappers: AddedMappers,
	removed_mappers: RemovedMappers,
	filter_updates: FilterUpdates,
}

#[derive(Debug, Builder)]
pub struct FilterUpdate
{
	#[builder(required)]
	nub_tier: Option<Tier>,
	#[builder(required)]
	pro_tier: Option<Tier>,
	#[builder(required)]
	ranked: Option<bool>,
	#[builder(required)]
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

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn update<I, AddedMappers, RemovedMappers, FilterUpdates>(
	#[builder(start_fn)] map_id: MapId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	workshop_id: WorkshopId,
	name: MapName,
	#[builder(required)] description: Option<MapDescription>,
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
	.execute(db_conn.raw_mut())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await?;

	debug!(%map_id, "updated map");

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
	.fetch(db_conn.raw_mut())
	.map_ok(|row| (row.local_id, row.id))
	.try_collect::<HashMap<CourseLocalId, CourseId>>()
	.await?;

	for (local_id, course_update) in course_updates {
		let &course_id = course_ids
			.get(&local_id)
			.ok_or(UpdateMapError::InvalidCourseLocalId(local_id))?;

		update_course(course_id)
			.name(course_update.name)
			.description(course_update.description)
			.added_mappers(course_update.added_mappers)
			.removed_mappers(course_update.removed_mappers)
			.filter_updates(course_update.filter_updates)
			.exec(&mut *db_conn)
			.await?;

		debug!(%local_id, "updated course");
	}

	Ok(())
}

#[instrument(level = "debug", skip(db_conn), err)]
#[builder(finish_fn = exec)]
async fn update_course<AddedMappers, RemovedMappers, FilterUpdates>(
	#[builder(start_fn)] course_id: CourseId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	#[builder(required)] name: Option<CourseName>,
	#[builder(required)] description: Option<CourseDescription>,
	added_mappers: AddedMappers,
	removed_mappers: RemovedMappers,
	filter_updates: FilterUpdates,
) -> DatabaseResult<()>
where
	AddedMappers: IntoIterator<Item = UserId> + fmt::Debug,
	RemovedMappers: IntoIterator<Item = UserId> + fmt::Debug,
	FilterUpdates: IntoIterator<Item = (Mode, FilterUpdate)> + fmt::Debug,
{
	sqlx::query!(
		"UPDATE Courses
		 SET name = COALESCE(?, name),
		     description = COALESCE(?, description)
		 WHERE id = ?",
		name,
		description,
		course_id,
	)
	.execute(db_conn.raw_mut())
	.await?;

	{
		let mut query = QueryBuilder::new("INSERT INTO CourseMappers (course_id, user_id)");
		let mut had_values = false;

		query.push_values(added_mappers, |mut query, user_id| {
			query.push_bind(user_id);
			had_values = true;
		});

		if had_values {
			query.build().execute(db_conn.raw_mut()).await?;
		}
	}

	{
		let mut query = QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ");
		query.push_bind(course_id);
		query.push(" AND user_id IN ");

		let mut had_values = false;

		query.push_tuples(removed_mappers, |mut query, user_id| {
			query.push_bind(user_id);
			had_values = true;
		});

		if had_values {
			query.build().execute(db_conn.raw_mut()).await?;
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
		.execute(db_conn.raw_mut())
		.await?;

		debug!(?mode, "updated filter");
	}

	Ok(())
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn set_state(
	#[builder(start_fn)] map_id: MapId,
	#[builder(start_fn)] state: MapState,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<bool>
{
	let updated = sqlx::query!(
		"UPDATE Maps
		 SET state = ?
		 WHERE id = ?",
		state,
		map_id,
	)
	.execute(db_conn.raw_mut())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.await?;

	if updated {
		event_queue::dispatch(Event::MapApproved { id: map_id });
	}

	Ok(updated)
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
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
	.execute(db_conn.raw_mut())
	.map_ok(|query_result| query_result.rows_affected())
	.map_err(DatabaseError::from)
	.await
}

#[instrument(ret(level = "debug"), err)]
pub async fn download_and_hash(
	workshop_id: WorkshopId,
	exe_path: &Path,
	out_dir: &Path,
) -> io::Result<Checksum>
{
	workshop::download(workshop_id, exe_path, out_dir)
		.await
		.inspect_err_dyn(|error| error!(error, "failed to download workshop map"))?;

	let span = tracing::info_span!("read_depot_downloader_result", ?out_dir);
	span.follows_from(tracing::Span::current());

	let out_dir = out_dir.to_owned();

	task::spawn_blocking(move || {
		let _guard = span.entered();
		let mut checksum = Checksum::builder();
		let out_dir_entries = fs::read_dir(&out_dir)
			.inspect_err_dyn(|error| error!(error, "failed to read directory"))?;

		for entry in out_dir_entries {
			let entry = entry.inspect_err_dyn(|error| {
				error!(error, "failed to read directory entry");
			})?;

			let filename = match entry.file_name().into_string() {
				Ok(name) => name,
				Err(name) => {
					warn!("entry {name:?} is not valid UTF-8?");
					continue;
				},
			};

			let Some((prefix, rest)) = filename.split_once('_') else {
				continue;
			};

			let Some((_, "vpk")) = rest.split_once('.') else {
				continue;
			};

			if !prefix.parse::<WorkshopId>().is_ok_and(|prefix| prefix == workshop_id) {
				continue;
			}

			let path = entry.path();
			let mut file = File::open(&path)
				.inspect_err_dyn(|error| error!(error, "failed to open {path:?}"))?;

			checksum
				.feed_reader(&mut file)
				.inspect_err_dyn(|error| error!(error, "failed to read {path:?}"))?;
		}

		Ok(checksum.build())
	})
	.await
	.unwrap_or_else(|err| panic!("{err}"))
}
