use {
	crate::{
		res::{maps as res, BadRequest},
		util::{self, Created},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{MapIdentifier, Mode, PlayerIdentifier, SteamID, Style, Tier},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::collections::HashMap,
	utoipa::{IntoParams, ToSchema},
};

const LIMIT_DEFAULT: u64 = 100;
const LIMIT_MAX: u64 = 1000;

const ROOT_GET_BASE_QUERY: &str = r#"
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

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMapsParams<'a> {
	pub name: Option<String>,
	pub created_by: Option<PlayerIdentifier<'a>>,
	pub created_after: Option<DateTime<Utc>>,
	pub created_before: Option<DateTime<Utc>>,

	#[serde(default)]
	pub offset: u64,
	pub limit: Option<u64>,
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
) -> Response<Vec<res::KZMap>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = util::Filter::new();

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
				sqlx::query!("SELECT id FROM Players WHERE name LIKE ?", name)
					.fetch_one(state.database())
					.await?
					.id
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

	let limit = limit.map_or(LIMIT_DEFAULT, |limit| std::cmp::min(limit, LIMIT_MAX));

	query
		.push(" LIMIT ")
		.push_bind(offset)
		.push(",")
		.push_bind(limit);

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
pub async fn get_map(state: State, Path(ident): Path<MapIdentifier<'_>>) -> Response<res::KZMap> {
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMap {
	name: String,
	workshop_id: u32,
	courses: Vec<Course>,
	filters: Vec<Filter>,
	filesize: u64,
	owned_by: SteamID,
	approved_by: SteamID,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Course {
	stage: u8,
	difficulty: Tier,
	created_by: SteamID,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Filter {
	stage: u8,
	mode: Mode,
	style: Style,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedMap {
	id: u16,
	courses: Vec<CreatedCourse>,
	filters: Vec<CreatedFilter>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedCourse {
	id: u32,
	stage: u8,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedFilter {
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
	Json(NewMap { name, workshop_id, courses, filters, filesize, owned_by, approved_by }): Json<
		NewMap,
	>,
) -> Result<Created<Json<CreatedMap>>> {
	validate_courses_and_filters(&courses, &filters)?;

	let mut transaction = state.database().begin().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Maps (name, workshop_id, filesize, owned_by)
		VALUES
			(?, ?, ?, ?)
		"#,
		name,
		workshop_id,
		filesize,
		owned_by.as_u32(),
	}
	.execute(transaction.as_mut())
	.await?;

	let map = sqlx::query!("SELECT * FROM Maps WHERE id = (SELECT MAX(id) id FROM Maps)")
		.fetch_one(transaction.as_mut())
		.await?;

	let mut create_courses =
		QueryBuilder::new("INSERT INTO Courses (map_id, stage, difficulty, created_by)");

	create_courses.push_values(&courses, |mut query, course| {
		query
			.push_bind(map.id)
			.push_bind(course.stage)
			.push_bind(course.difficulty as u8)
			.push_bind(course.created_by.as_u32());
	});

	create_courses
		.build()
		.execute(transaction.as_mut())
		.await?;

	let courses = sqlx::query!("SELECT * FROM Courses WHERE map_id = ?", map.id)
		.fetch_all(transaction.as_mut())
		.await?
		.into_iter()
		.map(|course| (course.stage, course))
		.collect::<HashMap<_, _>>();

	let min_course_id = courses.values().map(|course| course.id).min();
	let max_course_id = courses.values().map(|course| course.id).max();

	let mut create_filters =
		QueryBuilder::new("INSERT INTO Filters (course_id, mode_id, style_id)");

	create_filters.push_values(&filters, |mut query, filter| {
		let course_id = courses
			.get(&filter.stage)
			.map(|course| course.id)
			.expect("we made sure filters refer to valid courses");

		query
			.push_bind(course_id)
			.push_bind(filter.mode as u8)
			.push_bind(filter.style as u8);
	});

	create_filters
		.build()
		.execute(transaction.as_mut())
		.await?;

	transaction.commit().await?;

	let filters = sqlx::query!(
		"SELECT * FROM Filters where course_id < ? AND course_id > ?",
		min_course_id,
		max_course_id,
	)
	.fetch_all(state.database())
	.await?;

	let courses = courses
		.into_values()
		.map(|course| CreatedCourse { id: course.id, stage: course.stage })
		.collect::<Vec<_>>();

	let filters = filters
		.into_iter()
		.map(|filter| CreatedFilter {
			id: filter.id,
			filter: Filter {
				stage: courses
					.iter()
					.find(|course| course.id == filter.course_id)
					.map(|course| course.stage)
					.expect("we just inserted these courses"),
				mode: filter
					.mode_id
					.try_into()
					.expect("invalid mode in database"),
				style: filter
					.style_id
					.try_into()
					.expect("invalid filter in database"),
			},
		})
		.collect::<Vec<_>>();

	Ok(Created(Json(CreatedMap { id: map.id, courses, filters })))
}

fn validate_courses_and_filters(courses: &[Course], filters: &[Filter]) -> Result<()> {
	let mut stage_counters = vec![0; courses.len()];
	let course_has_stage = |stage: u8| courses.iter().any(|course| course.stage == stage);

	// Make sure courses have no gaps in their stages
	for stage in 0..courses.len() as u8 {
		if !course_has_stage(stage) {
			return Err(Error::MissingCourse { stage });
		}

		stage_counters[stage as usize] += 1;
	}

	// Make sure each stage was only submitted once
	for (stage, &count) in stage_counters.iter().enumerate() {
		if count > 1 {
			return Err(Error::DuplicateCourse { stage: stage as u8 });
		}
	}

	// Make sure each filter actually refers to an existing course
	for stage in filters.iter().map(|filter| filter.stage) {
		if !course_has_stage(stage) {
			return Err(Error::InvalidFilter { stage });
		}

		stage_counters[stage as usize] += 1;
	}

	// Make sure each filter was only submitted once
	for (stage, &count) in stage_counters.iter().enumerate() {
		if count > 2 {
			return Err(Error::DuplicateFilter { stage: stage as u8 });
		}
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MapUpdate {
	name: Option<String>,
	workshop_id: Option<u32>,
	filters_added: Option<Vec<FilterWithCourseId>>,
	filters_removed: Option<Vec<u32>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FilterWithCourseId {
	course_id: u32,
	mode: Mode,
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
	let mut transaction = state.database().begin().await?;

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
			sqlx::query!("DELETE FROM Filters WHERE id = ?", filter_id)
				.execute(transaction.as_mut())
				.await?;
		}
	}

	transaction.commit().await?;

	Ok(())
}
