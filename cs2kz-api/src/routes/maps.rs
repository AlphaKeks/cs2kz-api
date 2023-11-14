use {
	super::{BoundedU64, Created},
	crate::{
		res::{maps as res, BadRequest},
		Error, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{MapIdentifier, Mode, PlayerIdentifier, Runtype, SteamID, Style, Tier},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	utoipa::{IntoParams, ToSchema},
};

static ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		m.id,
		m.name,
		m.workshop_id,
		m.filesize,
		p1.name player_name,
		p1.id steam_id,
		m.created_on,
		c.id course_id,
		c.stage course_stage,
		c.difficulty course_tier,
		p2.id course_created_by_id,
		p2.name course_created_by_name
	FROM
		Maps m
		JOIN Players p1 ON p1.id = m.owned_by
		JOIN Courses c ON c.map_id = m.id
		JOIN Players p2 ON p2.id = c.created_by
"#;

/// Query parameters for fetching maps.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMapsParams<'a> {
	/// A map's name.
	pub name: Option<String>,

	/// A player's `SteamID` or name.
	pub created_by: Option<PlayerIdentifier<'a>>,

	/// Only include maps that were globalled after a certain date.
	pub created_after: Option<DateTime<Utc>>,

	/// Only include maps that were globalled before a certain date.
	pub created_before: Option<DateTime<Utc>>,

	#[param(value_type = Option<u64>, default = 0)]
	pub offset: BoundedU64,

	/// Return at most this many results.
	///
	/// Defaults to 100 and caps out at 1000.
	#[param(value_type = Option<u64>, default = 100, maximum = 1000)]
	pub limit: BoundedU64<100, 1000>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Maps", context_path = "/api/v0", path = "/maps",
	params(GetMapsParams),
	responses(
		(status = 200, body = Vec<KZMap>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_maps(
	state: State,
	Query(GetMapsParams { name, created_by, created_after, created_before, offset, limit }): Query<
		GetMapsParams<'_>,
	>,
) -> Result<Json<Vec<res::KZMap>>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = super::Filter::new();

	if let Some(ref name) = name {
		query
			.push(filter)
			.push(" m.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(player) = created_by {
		let steam32_id = match player {
			PlayerIdentifier::SteamID(steam_id) => steam_id.as_u32(),
			PlayerIdentifier::Name(name) => {
				sqlx::query!("SELECT steam_id FROM Players WHERE name LIKE ?", name)
					.fetch_one(state.database())
					.await?
					.steam_id
			}
		};

		query
			.push(filter)
			.push(" p1.id = ")
			.push_bind(steam32_id);

		filter.switch();
	}

	if let Some(created_after) = created_after {
		query
			.push(filter)
			.push(" m.created_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = created_before {
		query
			.push(filter)
			.push(" m.created_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	query.push(" GROUP BY m.id ");
	super::push_limit(&mut query, offset, limit);

	let maps = query
		.build_query_as::<res::KZMap>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.fold(Vec::<res::KZMap>::new(), |mut maps, mut map| {
			if let Some(last_map) = maps.last_mut() {
				if last_map.id == map.id {
					last_map.courses.append(&mut map.courses);
					return maps;
				}
			};

			maps.push(map);
			maps
		});

	if maps.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(maps))
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Maps", context_path = "/api/v0", path = "/maps/{ident}",
	params(("ident" = MapIdentifier, Path, description = "The map's ID or name")),
	responses(
		(status = 200, body = KZMap),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_map(
	state: State,
	Path(ident): Path<MapIdentifier<'_>>,
) -> Result<Json<res::KZMap>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		MapIdentifier::ID(id) => {
			query.push(" m.id = ").push_bind(id);
		}
		MapIdentifier::Name(name) => {
			query
				.push(" m.name LIKE ")
				.push_bind(format!("%{name}%"));
		}
	};

	let map = query
		.build_query_as::<res::KZMap>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.reduce(|mut acc, mut row| {
			acc.courses.append(&mut row.courses);
			acc
		})
		.ok_or(Error::NoContent)?;

	Ok(Json(map))
}

/// Information about a new KZ map.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMap {
	/// The name of the map.
	name: String,

	/// The Steam workshop ID of the map.
	workshop_id: u32,

	/// A list of the map's courses.
	courses: Vec<Course>,

	/// The filesize of the map.
	filesize: u64,

	/// The `SteamID` of the player who published this map.
	created_by: SteamID,

	/// The `SteamID` of the admin who approved this map to be globalled.
	approved_by: SteamID,
}

/// A course on a KZ map.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Course {
	/// The stage this course corresponds to.
	map_stage: u8,

	/// The `SteamID` of the player who created this course.
	created_by: SteamID,

	/// List of filters on this course.
	filters: Vec<Filter>,
}

/// A filter for a KZ map course.
///
/// It describes which combination of mode and style are allowed to submit records, and how
/// difficult it is to complete a course with that combination.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Filter {
	/// A KZ mode.
	mode: Mode,

	/// Whether teleports can be used.
	runtype: Runtype,

	/// A difficulty rating.
	tier: Tier,
}

/// Information about a newly created KZ map.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedMap {
	/// The ID of the map.
	id: u16,

	/// List of courses.
	courses: Vec<CreatedCourse>,
}

/// A newly created course on a KZ map.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedCourse {
	/// The ID of the course.
	id: u32,

	/// The stage this course corresponds to.
	map_stage: u8,

	/// A list of filters on this course.
	filters: Vec<CreatedFilter>,
}

/// A newly created filter on a KZ map course.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedFilter {
	/// The ID of the filter.
	id: u32,

	#[serde(flatten)]
	filter: Filter,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Maps", context_path = "/api/v0", path = "/maps",
	request_body = NewMap,
	responses(
		(status = 201, body = CreatedMap),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_map(
	state: State,
	Json(NewMap { name, workshop_id, courses, filesize, created_by, approved_by }): Json<NewMap>,
) -> Result<Created<Json<CreatedMap>>> {
	validate_courses(&courses)?;

	let mut transaction = state.transaction().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Maps (name, workshop_id, filesize, created_by)
		VALUES
			(?, ?, ?, ?)
		"#,
		name,
		workshop_id,
		filesize,
		created_by.as_u32(),
	}
	.execute(transaction.as_mut())
	.await?;

	let map = sqlx::query!("SELECT * FROM Maps WHERE id = (SELECT MAX(id) id FROM Maps)")
		.fetch_one(transaction.as_mut())
		.await?;

	let mut create_courses = QueryBuilder::new("INSERT INTO Courses (map_id, stage, created_by)");

	create_courses.push_values(&courses, |mut query, course| {
		query
			.push_bind(map.id)
			.push_bind(course.map_stage)
			.push_bind(course.created_by.as_u32());
	});

	create_courses
		.build()
		.execute(transaction.as_mut())
		.await?;

	let db_courses = sqlx::query!("SELECT * FROM Courses WHERE map_id = ? ORDER BY id ASC", map.id)
		.fetch_all(transaction.as_mut())
		.await?;

	let mut create_filters =
		QueryBuilder::new("INSERT INTO CourseFilters (course_id, mode_id, has_teleports, tier)");

	create_filters.push_values(
		courses.iter().flat_map(|course| {
			course.filters.iter().map(|filter| {
				let course = db_courses
					.iter()
					.find(|c| c.map_stage == course.map_stage)
					.expect("we just inserted all the courses");

				(course.id, filter)
			})
		}),
		|mut query, (course_id, filter)| {
			query
				.push_bind(course_id)
				.push_bind(filter.mode as u8)
				.push_bind(bool::from(filter.runtype))
				.push_bind(filter.tier as u8);
		},
	);

	create_filters
		.build()
		.execute(transaction.as_mut())
		.await?;

	transaction.commit().await?;

	let db_filters = sqlx::query!(
		r#"
		SELECT
			f.*
		FROM
			CourseFilters f
			JOIN Courses c ON c.id = f.course_id
		WHERE
			c.map_id = ?
		"#,
		map.id,
	)
	.fetch_all(state.database())
	.await?;

	let courses = db_courses
		.into_iter()
		.map(|course| CreatedCourse {
			id: course.id,
			map_stage: course.map_stage,
			filters: db_filters
				.iter()
				.filter(|filter| filter.course_id == course.id)
				.map(|filter| CreatedFilter {
					id: filter.id,
					filter: Filter {
						mode: filter
							.mode_id
							.try_into()
							.expect("invalid mode in database"),
						runtype: (filter.has_teleports == 1).into(),
						tier: filter
							.tier
							.try_into()
							.expect("invalid tier in database"),
					},
				})
				.collect(),
		})
		.collect();

	Ok(Created(Json(CreatedMap { id: map.id, courses })))
}

/// Makes sure courses correspond to valid stages.
fn validate_courses(courses: &[Course]) -> Result<()> {
	let mut counters = vec![0_usize; courses.len()];

	for course in courses {
		counters[course.map_stage as usize] += 1;
	}

	for (stage, &count) in counters.iter().enumerate() {
		let stage = stage as u8;

		if count == 0 {
			return Err(Error::MissingCourse { stage });
		}

		if count > 1 {
			return Err(Error::DuplicateCourse { stage });
		}
	}

	Ok(())
}

/// Updated information about a KZ map.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MapUpdate {
	/// The new name of the map.
	name: Option<String>,

	/// The new Steam workshop ID of the map.
	workshop_id: Option<u32>,

	/// A list of new additional filters.
	filters_added: Option<Vec<FilterWithCourseId>>,

	/// A list of IDs for filters that should be removed.
	filters_removed: Option<Vec<u32>>,
}

/// Information about a filter.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FilterWithCourseId {
	/// The ID of the course this filter belongs to.
	course_id: u32,

	/// A KZ mode.
	mode: Mode,

	/// A KZ style.
	style: Style,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Maps", context_path = "/api/v0", path = "/maps/{id}",
	params(("id" = u16, Path, description = "The map's ID")),
	request_body = MapUpdate,
	responses(
		(status = 200),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn update_map(
	state: State,
	Path(map_id): Path<u16>,
	Json(MapUpdate { name, workshop_id, filters_added, filters_removed }): Json<MapUpdate>,
) -> Result<()> {
	let mut transaction = state.transaction().await?;

	if let Some(name) = name {
		sqlx::query!("UPDATE Maps SET name = ? WHERE id = ?", name, map_id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(workshop_id) = workshop_id {
		sqlx::query!("UPDATE Maps SET workshop_id = ? WHERE id = ?", workshop_id, map_id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(filters) = filters_added {
		let mut create_filters =
			QueryBuilder::new("INSERT INTO Filters (course_id, mode_id, style_id)");

		create_filters.push_values(filters, |mut query, filter| {
			query
				.push_bind(filter.course_id)
				.push_bind(filter.mode as u8)
				.push_bind(filter.style as u8);
		});

		create_filters
			.build()
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(filters) = filters_removed {
		for filter_id in filters {
			sqlx::query!("DELETE FROM CourseFilters WHERE id = ?", filter_id)
				.execute(transaction.as_mut())
				.await?;
		}
	}

	transaction.commit().await?;

	Ok(())
}
