use {
	crate::{
		Config,
		TaskManager,
		http::{
			auth,
			extract::{header::Header, path::Path, query::Query},
			json::Json,
			openapi,
			pagination::{Limit, Offset, PaginationResponse},
			problem_details::{ProblemDescription, ProblemDetails, ProblemType},
			response::{Created, HandlerError, HandlerResult},
		},
		runtime,
	},
	axum::{
		extract::State,
		response::{IntoResponse, NoContent, Redirect, Response, Sse, sse},
	},
	axum_extra::extract::CookieJar,
	axum_tws::WebSocketUpgrade,
	cs2kz_api::{
		access_keys::AccessKey,
		bans::{self, Ban, BanId, BanReason, CreateBanError, CreatedBan, UnbanReason},
		checksum::Checksum,
		database,
		email::EmailAddress,
		error::ResultExt,
		event_queue::{self, Event},
		game::Game,
		git_revision::GitRevision,
		maps::{
			self,
			CourseDescription,
			CourseId,
			CourseLocalId,
			CourseName,
			CourseUpdate,
			CreateMapError,
			FilterNotes,
			FilterUpdate,
			Map,
			MapDescription,
			MapId,
			MapName,
			MapState,
			NewCS2Filters,
			NewCSGOFilters,
			NewCourse,
			NewFilter,
			NewFilters,
			Tier,
		},
		mode::Mode,
		players::{self, Player, PlayerId, PlayerPreferences, PlayerRating},
		plugin::{self, PluginVersion, PluginVersionInfo},
		records::{self, Leaderboard, Record, RecordId},
		server_monitor::{self, ServerMonitorHandle},
		servers::{
			self,
			CreateServerError,
			CreatedServer,
			Server,
			ServerHost,
			ServerId,
			ServerName,
			ServerPort,
		},
		steam::{
			self,
			workshop::{self, WorkshopId},
		},
		stream::{StreamExt as _, TryStreamExt as _},
		styles::Style,
		time::{Seconds, Timestamp},
		users::{
			self,
			Permission,
			Permissions,
			ServerBudget,
			User,
			UserId,
			Username,
			sessions::SessionId,
		},
	},
	futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt as _, stream},
	headers::{Authorization, authorization::Bearer},
	serde::{Deserialize, Serialize},
	std::{
		collections::{BTreeMap, BTreeSet, HashMap},
		error::Error,
		pin::pin,
		sync::{Arc, LazyLock},
	},
	steam_openid::VerifyCallbackPayloadErrorKind,
	tokio_util::sync::CancellationToken,
	url::Url,
	utoipa::{IntoParams, ToSchema},
};

const PLAYER_COOKIE_NAME: &str = "kz-player";

//=================================================================================================
// `/docs`

#[instrument(level = "trace", skip(config))]
pub(crate) async fn openapi_json(State(config): State<Arc<Config>>) -> Response
{
	let schema = openapi::SCHEMA.get_or_init(|| {
		let mut schema = openapi::schema();

		if runtime::environment::get().is_development() | runtime::environment::get().is_testing() {
			let staging_server = utoipa::openapi::ServerBuilder::default()
				.url("https://testing.cs2kz.org")
				.description(Some("test instance"))
				.build();

			schema.servers.get_or_insert_default().insert(0, staging_server);
		}

		if runtime::environment::get().is_development() {
			let local_server = utoipa::openapi::ServerBuilder::default()
				.url(config.http.public_url.as_str())
				.description(Some("local dev server"))
				.build();

			schema.servers.get_or_insert_default().insert(0, local_server);
		}

		schema
	});

	Json(schema).into_response()
}

#[instrument(level = "trace")]
pub(crate) async fn problems_json() -> Response
{
	static PROBLEMS: LazyLock<Box<[ProblemDescription]>> =
		LazyLock::new(|| ProblemType::ALL.iter().map(ProblemType::description).collect());

	Json(&PROBLEMS[..]).into_response()
}

#[instrument(level = "trace")]
pub(crate) async fn swagger_ui(path: Option<Path<String>>) -> Response
{
	static CONFIG: LazyLock<Arc<utoipa_swagger_ui::Config<'static>>> = LazyLock::new(|| {
		let cfg = utoipa_swagger_ui::Config::from("/docs/openapi.json")
			.display_operation_id(true)
			.use_base_layout()
			.display_request_duration(true)
			.filter(true)
			.request_snippets_enabled(true)
			.with_credentials(true);

		Arc::new(cfg)
	});

	let tail = match path {
		None => "",
		Some(Path(ref path)) => path.as_str(),
	};

	match utoipa_swagger_ui::serve(tail, Arc::clone(&*CONFIG)) {
		Ok(None) => http::StatusCode::NOT_FOUND.into_response(),
		Ok(Some(file)) => Response::builder()
			.header(http::header::CONTENT_TYPE, file.content_type)
			.body(file.bytes.into())
			.unwrap_or_else(|err| panic!("failed to build hard-coded response: {err}")),
		Err(err) => {
			error!(error = &*err, "failed to serve SwaggerUI file");
			http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
		},
	}
}

//=================================================================================================
// `/leaderboards`

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetRatingLeaderboardQuery
{
	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
pub(crate) struct RatingLeaderboard(Vec<players::RatingLeaderboardEntry>);

/// Global Player Rating Leaderboard
///
/// This endpoint returns the highest rated players in KZ.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/leaderboards/rating",
	tag = "Leaderboards",
	params(GetRatingLeaderboardQuery),
	responses(
		(status = 200, body = RatingLeaderboard),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_rating_leaderboard(
	State(database): State<database::ConnectionPool>,
	Query(GetRatingLeaderboardQuery { limit }): Query<GetRatingLeaderboardQuery>,
) -> HandlerResult<Json<RatingLeaderboard>>
{
	let mut db_conn = database.acquire().await?;
	let entries = players::get_rating_leaderboard()
		.size(limit.value())
		.exec(&mut db_conn)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(Json(RatingLeaderboard(entries)))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetRecordsLeaderboardQuery
{
	/// Only count records for a specific mode
	mode: Option<Mode>,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(transparent)]
pub(crate) struct RecordsLeaderboard(Vec<players::RecordsLeaderboardEntry>);

/// Global World Record Leaderboard
///
/// This endpoint returns the players with the most World Records.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/leaderboards/records/{leaderboard}",
	tag = "Leaderboards",
	params(GetRecordsLeaderboardQuery, ("leaderboard" = Leaderboard, Path)),
	responses(
		(status = 200, body = RecordsLeaderboard),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_records_leaderboard(
	State(database): State<database::ConnectionPool>,
	Path(leaderboard): Path<Leaderboard>,
	Query(GetRecordsLeaderboardQuery { mode, limit }): Query<GetRecordsLeaderboardQuery>,
) -> HandlerResult<Json<RecordsLeaderboard>>
{
	let mut db_conn = database.acquire().await?;
	let entries = players::get_records_leaderboard(leaderboard)
		.maybe_mode(mode)
		.size(limit.value())
		.exec(&mut db_conn)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(Json(RecordsLeaderboard(entries)))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetCourseLeaderboardQuery
{
	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

/// Course Leaderboards
///
/// This endpoint returns the leaderboard for a specific course in a specific
/// mode.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/leaderboards/course/{course_id}/{mode}/{leaderboard}",
	tag = "Leaderboards",
	params(
		("course_id" = CourseId, Path),
		("mode" = Mode, Path),
		("leaderboard" = Leaderboard, Path),
		GetCourseLeaderboardQuery,
	),
	responses(
		(status = 200, body = [Record]),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
	),
)]
pub(crate) async fn get_course_leaderboard(
	State(database): State<database::ConnectionPool>,
	Path((course_id, mode, leaderboard)): Path<(CourseId, Mode, Leaderboard)>,
	Query(GetCourseLeaderboardQuery { limit }): Query<GetCourseLeaderboardQuery>,
) -> HandlerResult<Json<Vec<Record>>>
{
	let mut db_conn = database.acquire().await?;

	let filter_id = maps::get_filter_id(course_id, mode)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	let records = records::get_detailed_leaderboard(filter_id, leaderboard)
		.size(limit.value())
		.exec(&mut db_conn)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(Json(records))
}

//=================================================================================================
// `/records`

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetRecordsQuery
{
	/// Only include records set by this player
	player: Option<PlayerId>,

	/// Only include records set on this course
	course: Option<CourseId>,

	/// Only include records set on this mode
	mode: Option<Mode>,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

/// Records
///
/// This endpoint returns the latest records.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/records",
	tag = "Records",
	params(GetRecordsQuery),
	responses(
		(status = 200, body = PaginationResponse<Record>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_records(
	State(database): State<database::ConnectionPool>,
	Query(GetRecordsQuery { player, course, mode, offset, limit }): Query<GetRecordsQuery>,
) -> HandlerResult<Json<PaginationResponse<Record>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response = PaginationResponse::new({
		records::count()
			.maybe_player(player)
			.maybe_course(course)
			.maybe_mode(mode)
			.exec(&mut db_conn)
			.await?
	});

	records::get()
		.maybe_player(player)
		.maybe_course(course)
		.maybe_mode(mode)
		.offset(offset.value())
		.limit(limit.value())
		.exec(&mut db_conn)
		.try_collect_into(&mut response)
		.await?;

	Ok(Json(response))
}

/// Records by ID
///
/// This endpoint returns a specific record by its API-assigned ID.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/records/{record_id}",
	tag = "Records",
	params(("record_id" = RecordId, Path)),
	responses(
		(status = 200, body = Record),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_record(
	State(database): State<database::ConnectionPool>,
	Path(record_id): Path<RecordId>,
) -> HandlerResult<Json<Record>>
{
	let mut db_conn = database.acquire().await?;
	let record = records::get_by_id(record_id)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(record))
}

//=================================================================================================
// `/maps`

#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "workshop_id": 3121168339_u32,
  "description": "KZ but in a GROTTO! Make your way through an obstacle course based in a cave.",
  "courses": [
    {
      "name": "Main",
      "mappers": ["76561198260657129"],
      "filters": {
        "cs2": {
          "vnl": {
            "nub_tier": "medium",
            "pro_tier": "advanced",
            "ranked": true
		  },
          "ckz": {
            "nub_tier": "easy",
            "pro_tier": "medium",
            "ranked": true
		  }
		}
	  }
    },
    {
      "name": "Garden",
      "mappers": ["76561198260657129"],
      "filters": {
        "cs2": {
          "vnl": {
            "nub_tier": "easy",
            "pro_tier": "easy",
            "ranked": true
		  },
          "ckz": {
            "nub_tier": "easy",
            "pro_tier": "easy",
            "ranked": true
		  }
		}
	  }
    },
    {
      "name": "word's backyard",
      "mappers": ["76561198260657129"],
      "filters": {
        "vnl": {
          "nub_tier": "hard",
          "pro_tier": "very-hard",
          "ranked": true
		},
        "ckz": {
          "nub_tier": "advanced",
          "pro_tier": "advanced",
          "ranked": true
		}
	  }
    },
    {
      "name": "Old grotto (hard)",
      "mappers": ["76561198260657129"],
      "filters": {
        "vnl": {
          "nub_tier": "very-hard",
          "pro_tier": "death",
          "ranked": true
		},
        "ckz": {
          "nub_tier": "medium",
          "pro_tier": "advanced",
          "ranked": true
		}
	  }
    }
  ]
}))]
pub(crate) struct CreateMapRequest
{
	workshop_id: WorkshopId,

	#[serde(default)]
	description: MapDescription,

	game: Game,

	#[serde(deserialize_with = "cs2kz_api::serde::de::non_empty")]
	courses: Vec<CreateCourseRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateCourseRequest
{
	name: CourseName,

	#[serde(default)]
	description: CourseDescription,

	#[serde(deserialize_with = "cs2kz_api::serde::de::non_empty")]
	mappers: BTreeSet<UserId>,

	filters: CreateFiltersRequest,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub(crate) enum CreateFiltersRequest
{
	CS2
	{
		vnl: CreateFilterRequest, ckz: CreateFilterRequest
	},

	#[allow(clippy::upper_case_acronyms)]
	CSGO
	{
		kzt: CreateFilterRequest, skz: CreateFilterRequest, vnl: CreateFilterRequest
	},
}

impl CreateFiltersRequest
{
	fn game(&self) -> Game
	{
		match *self {
			Self::CS2 { .. } => Game::CS2,
			Self::CSGO { .. } => Game::CSGO,
		}
	}
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateFilterRequest
{
	nub_tier: Tier,
	pro_tier: Tier,
	ranked: bool,

	#[serde(default)]
	notes: FilterNotes,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CreateMapResponse
{
	map_id: MapId,
}

/// Map Submission
///
/// This endpoint can be used to submit KZ maps to the API. All maps must be
/// uploaded to Steam's Community Workshop and the API will source their name
/// from there. If you plan on submitting your map for approval, make sure it
/// follows [the rules].
///
/// [the rules]: https://docs.cs2kz.org/mapping/approval#rules
#[instrument(
	skip(config, database, steam_api_client),
	ret(level = "debug"),
	err(Debug, level = "debug")
)]
#[utoipa::path(
	put,
	path = "/maps",
	tag = "Maps",
	security(("session_auth" = [])),
	request_body = CreateMapRequest,
	responses(
		(status = 201, body = CreateMapResponse),
		(status = 401,),
		(status = 409, body = ProblemDetails, description = "map properties conflict with existing map(s) or are logically invalid"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn create_map(
	State(config): State<Arc<Config>>,
	State(database): State<database::ConnectionPool>,
	State(steam_api_client): State<steam::api::Client>,
	session: auth::Session,
	Json(CreateMapRequest { workshop_id, description, game, courses }): Json<CreateMapRequest>,
) -> HandlerResult<Created<CreateMapResponse>>
{
	if !session.user_info().permissions().contains(&Permission::CreateMaps) {
		debug!("user does not have permissions");
		return Err(HandlerError::Unauthorized);
	}

	if let Some(course_local_id) = courses.iter().position(|course| course.filters.game() != game) {
		debug!("invalid filters");
		let mut problem_details = ProblemDetails::new(ProblemType::InconsistentFilters);
		problem_details.set_detail(format!(
			"expected filters for {game} but course {course_local_id} does not have such filters"
		));
		problem_details.add_extension_member("expected_game", &game);
		problem_details.add_extension_member("offending_course_local_id", &(course_local_id + 1));
		return Err(problem_details.into());
	}

	let map_metadata = workshop::get_map_metadata(&steam_api_client, workshop_id)
		.await?
		.ok_or_else(|| {
			let mut problem_details = ProblemDetails::new(ProblemType::InvalidWorkshopId);
			problem_details.set_detail("map not found on workshop");
			HandlerError::Problem(problem_details)
		})?;

	#[expect(clippy::collapsible_else_if)]
	if runtime::environment::get().is_development() {
		warn!("not checking whether the user is the actual mapper");
	} else {
		if map_metadata.creator_id != *session.user_info().id().as_ref() {
			debug!("user is not the mapper");
			return Err(HandlerError::Unauthorized);
		}
	}

	let map_name = map_metadata.name.parse::<MapName>().map_err(|err| {
		let mut problem_details = ProblemDetails::new(ProblemType::InvalidMapName);
		problem_details.set_detail(err.to_string());
		HandlerError::Problem(problem_details)
	})?;

	let vpk_checksum = maps::download_and_hash(
		workshop_id,
		&config.depot_downloader.exe_path,
		&config.depot_downloader.out_dir,
	)
	.await
	.map_err(|_| HandlerError::Internal)?;

	let map_id = database
		.in_transaction(async move |db_conn| {
			for course in &courses {
				let mappers = steam::users::get_many(
					&steam_api_client,
					course.mappers.iter().map(|user_id| *user_id.as_ref()),
				)
				.map_err(CreateMapError::SteamApiError)
				.await?;

				for &mapper_id in &course.mappers {
					let mapper = mappers
						.iter()
						.find(|mapper| mapper.id == *mapper_id.as_ref())
						.ok_or(CreateMapError::InvalidMapperId(mapper_id))?;

					let mapper_name = mapper.name.parse::<Username>().map_err(|err| {
						CreateMapError::InvalidMapperName { id: mapper_id, error: err }
					})?;

					if let Ok(()) =
						users::create(mapper_id).name(mapper_name).exec(&mut *db_conn).await
					{
						debug!(
							id = %mapper_id,
							name = mapper.name.as_str(),
							"registered mapper as new user",
						);
					}
				}
			}

			let courses = courses.into_iter().enumerate().map(|(idx, course)| {
				let local_id = u16::try_from(idx + 1).unwrap_or_else(|_| {
					unreachable!("we would have run out of memory by now");
				});
				let local_id = CourseLocalId::new(local_id);
				let (cs2_filters, csgo_filters) = match course.filters {
					CreateFiltersRequest::CS2 { vnl, ckz } => {
						let cs2_filters = NewCS2Filters::builder()
							.vnl({
								NewFilter::builder()
									.nub_tier(vnl.nub_tier)
									.pro_tier(vnl.pro_tier)
									.ranked(vnl.ranked)
									.notes(vnl.notes)
									.build()
							})
							.ckz({
								NewFilter::builder()
									.nub_tier(ckz.nub_tier)
									.pro_tier(ckz.pro_tier)
									.ranked(ckz.ranked)
									.notes(ckz.notes)
									.build()
							})
							.build();

						(Some(cs2_filters), None)
					},
					CreateFiltersRequest::CSGO { kzt, skz, vnl } => {
						let csgo_filters = NewCSGOFilters::builder()
							.kzt({
								NewFilter::builder()
									.nub_tier(kzt.nub_tier)
									.pro_tier(kzt.pro_tier)
									.ranked(kzt.ranked)
									.notes(kzt.notes)
									.build()
							})
							.skz({
								NewFilter::builder()
									.nub_tier(skz.nub_tier)
									.pro_tier(skz.pro_tier)
									.ranked(skz.ranked)
									.notes(skz.notes)
									.build()
							})
							.vnl({
								NewFilter::builder()
									.nub_tier(vnl.nub_tier)
									.pro_tier(vnl.pro_tier)
									.ranked(vnl.ranked)
									.notes(vnl.notes)
									.build()
							})
							.build();

						(None, Some(csgo_filters))
					},
				};

				let filters = NewFilters::builder()
					.maybe_csgo(csgo_filters)
					.maybe_cs2(cs2_filters)
					.build();

				NewCourse::builder(local_id)
					.name(course.name)
					.description(course.description)
					.mappers(course.mappers)
					.filters(filters)
					.build()
			});

			maps::create(workshop_id)
				.name(map_name)
				.description(description)
				.game(game)
				.state(MapState::WIP)
				.checksum(vpk_checksum)
				.created_by(session.user_info().id())
				.courses(courses)
				.exec(db_conn)
				.await
		})
		.await?;

	Ok(Created(CreateMapResponse { map_id }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetMapsQuery
{
	/// Only include maps with a matching name
	///
	/// If this parameter is specified, the returned maps will be ordered by how
	/// close their actual name matches the given value.
	name: Option<Box<str>>,

	/// Only include maps made for this game
	#[serde(default)]
	#[param(default)]
	game: Game,

	/// Only include maps in this state
	state: Option<MapState>,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<1000, 1000>,
}

/// Global KZ Maps
///
/// This endpoint returns the latest KZ maps.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/maps/{game}",
	tag = "Maps",
	params(GetMapsQuery),
	responses(
		(status = 200, body = PaginationResponse<Map>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_maps(
	State(database): State<database::ConnectionPool>,
	Query(GetMapsQuery { name, game, state, offset, limit }): Query<GetMapsQuery>,
) -> HandlerResult<Json<PaginationResponse<Map>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response = PaginationResponse::new({
		maps::count(game)
			.maybe_name(name.as_deref())
			.maybe_state(state)
			.exec(&mut db_conn)
			.await?
	});

	maps::get(game)
		.maybe_name(name.as_deref())
		.maybe_state(state)
		.offset(offset.value())
		.limit(limit.value())
		.exec(&mut db_conn)
		.try_collect_into(&mut response)
		.await?;

	Ok(Json(response))
}

/// Global KZ Maps by ID
///
/// This endpoint returns a specific KZ map by its API-assigned ID.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/maps/{map_id}",
	tag = "Maps",
	params(("map_id" = MapId, Path)),
	responses(
		(status = 200, body = Map),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_map(
	State(database): State<database::ConnectionPool>,
	Path(map_id): Path<MapId>,
) -> HandlerResult<Json<Map>>
{
	let mut db_conn = database.acquire().await?;
	let map = maps::get_by_id(map_id)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(map))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateMapRequest
{
	/// The ID of the map on the Steam Workshop
	workshop_id: WorkshopId,

	/// A new description
	description: Option<MapDescription>,

	/// Updates for the map's courses
	#[serde(default)]
	course_updates: BTreeMap<CourseLocalId, UpdateCourseRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateCourseRequest
{
	/// A new name
	name: Option<CourseName>,

	/// A new description
	description: Option<CourseDescription>,

	/// A list of SteamIDs to add as mappers
	#[serde(default)]
	added_mappers: BTreeSet<UserId>,

	/// A list of SteamIDs to remove as mappers
	#[serde(default)]
	removed_mappers: BTreeSet<UserId>,

	/// Updates to the course's filters
	#[serde(default)]
	filter_updates: HashMap<Mode, UpdateFiltersRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateFiltersRequest
{
	/// A new NUB tier
	nub_tier: Option<Tier>,

	/// A new PRO tier
	pro_tier: Option<Tier>,

	/// Whether the filter should contribute to player rating
	ranked: Option<bool>,

	/// Any additional notes
	notes: Option<FilterNotes>,
}

/// Update your Map
///
/// This endpoint can be used to make the API aware of changes to your map as
/// well as update metadata such as mapper information or descriptions. If your
/// map is currently work-in-progress and you uploaded a new version to Steam's
/// Community Workshop, you must send a request to this endpoint to make the API
/// aware of it.
#[instrument(
	skip(config, database, steam_api_client),
	ret(level = "debug"),
	err(Debug, level = "debug")
)]
#[utoipa::path(
	patch,
	path = "/maps/{map_id}",
	tag = "Maps",
	security(("session_auth" = ["update-maps"])),
	params(("map_id" = MapId, Path)),
	request_body = UpdateMapRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 403,),
		(status = 404,),
		(status = 409, body = ProblemDetails, description = "map properties conflict with existing map(s) or are logically invalid"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_map(
	State(config): State<Arc<Config>>,
	State(database): State<database::ConnectionPool>,
	State(steam_api_client): State<steam::api::Client>,
	session: auth::Session,
	Path(map_id): Path<MapId>,
	Json(UpdateMapRequest { workshop_id, description, course_updates }): Json<UpdateMapRequest>,
) -> HandlerResult<NoContent>
{
	let has_permissions = session.user_info().permissions().contains(&Permission::UpdateMaps);
	let mut db_conn = None;

	if !has_permissions {
		debug!("user does not have permissions");

		let db_conn = db_conn.insert(database.acquire().await?);
		let metadata = maps::get_metadata(map_id)
			.exec(db_conn)
			.await?
			.ok_or(HandlerError::NotFound)?;

		if metadata.created_by != session.user_info().id() {
			debug!("user is not the mapper");
			return Err(HandlerError::Unauthorized);
		}

		let state @ (MapState::Graveyard | MapState::WIP) = metadata.state else {
			debug!(state = ?metadata.state, "user cannot update frozen map");

			let mut problem_details = ProblemDetails::new(ProblemType::MapIsFrozen);
			problem_details.add_extension_member("map_state", &metadata.state);
			problem_details.set_detail(match metadata.state {
				MapState::Graveyard | MapState::WIP => unreachable!(),
				MapState::Pending => {
					"you already submitted the map for approval and have to wait for a decision \
					 before you can update it again"
				},
				MapState::Approved => "your map has already been approved",
				MapState::Completed => "you have already marked your map as 'completed'",
			});

			return Err(HandlerError::Problem(problem_details));
		};

		debug!(?state, "user is updating their map");
	}

	let (metadata, checksum) = {
		let metadata = workshop::get_map_metadata(&steam_api_client, workshop_id)
			.await?
			.ok_or_else(|| {
				let mut problem_details = ProblemDetails::new(ProblemType::InvalidWorkshopId);
				problem_details.set_detail("map not found on workshop");
				HandlerError::Problem(problem_details)
			})?;

		let checksum = maps::download_and_hash(
			workshop_id,
			&config.depot_downloader.exe_path,
			&config.depot_downloader.out_dir,
		)
		.await
		.map_err(|_| HandlerError::Internal)?;

		(metadata, checksum)
	};

	if !has_permissions && metadata.creator_id != *session.user_info().id().as_ref() {
		debug!("user is not the mapper");
		return Err(HandlerError::Unauthorized);
	}

	let map_name = metadata.name.parse::<MapName>().map_err(|err| {
		let mut problem_details = ProblemDetails::new(ProblemType::InvalidMapName);
		problem_details.set_detail(err.to_string());
		HandlerError::Problem(problem_details)
	})?;

	let db_conn = match db_conn {
		Some(ref mut db_conn) => db_conn,
		None => db_conn.insert(database.acquire().await?),
	};

	db_conn
		.in_transaction(async |conn| {
			let course_updates = course_updates.into_iter().map(|(local_id, course_update)| {
				let filter_updates =
					course_update.filter_updates.into_iter().map(|(mode, filter_update)| {
						let filter_update = FilterUpdate::builder()
							.nub_tier(filter_update.nub_tier)
							.pro_tier(filter_update.pro_tier)
							.ranked(filter_update.ranked)
							.notes(filter_update.notes)
							.build();

						(mode, filter_update)
					});

				let course_update = CourseUpdate::builder()
					.name(course_update.name)
					.description(course_update.description)
					.added_mappers(course_update.added_mappers)
					.removed_mappers(course_update.removed_mappers)
					.filter_updates(filter_updates)
					.build();

				(local_id, course_update)
			});

			maps::update(map_id)
				.workshop_id(workshop_id)
				.name(map_name)
				.description(description)
				.checksum(checksum)
				.course_updates(course_updates)
				.exec(conn)
				.await
		})
		.await?;

	Ok(NoContent)
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateMapStateRequest
{
	state: MapState,
}

/// Update a map's state
///
/// This endpoint can be used by the Map Approval Team to approve or reject
/// submitted maps.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/maps/{map_id}/state",
	tag = "Maps",
	security(("session_auth" = ["update-maps"])),
	params(("map_id" = MapId, Path)),
	request_body = UpdateMapStateRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 409, body = ProblemDetails, description = "map properties conflict with existing map(s) or are logically invalid"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_map_state(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(map_id): Path<MapId>,
	Json(UpdateMapStateRequest { state }): Json<UpdateMapStateRequest>,
) -> HandlerResult<NoContent>
{
	let mut db_conn = None;

	if !session.user_info().permissions().contains(&Permission::UpdateMaps) {
		debug!("user does not have permissions");

		let metadata = maps::get_metadata(map_id)
			.exec(db_conn.insert(database.acquire().await?))
			.await?
			.ok_or(HandlerError::NotFound)?;

		if metadata.created_by != session.user_info().id() {
			debug!("user is not the mapper");
			return Err(HandlerError::Unauthorized);
		}

		let state @ (MapState::Graveyard | MapState::WIP) = metadata.state else {
			debug!(state = ?metadata.state, "user cannot update frozen map's state");

			let mut problem_details = ProblemDetails::new(ProblemType::MapIsFrozen);
			problem_details.add_extension_member("map_state", &metadata.state);
			problem_details.set_detail(match metadata.state {
				MapState::Graveyard | MapState::WIP => unreachable!(),
				MapState::Pending => {
					"you already submitted the map for approval and have to wait for a decision \
					 before you can update it again"
				},
				MapState::Approved => "your map has already been approved",
				MapState::Completed => "you have already marked your map as 'completed'",
			});

			return Err(HandlerError::Problem(problem_details));
		};

		debug!(?state, "user is updating their map");
	}

	let db_conn = match db_conn {
		Some(ref mut conn) => conn,
		None => db_conn.insert(database.acquire().await?),
	};

	let updated = db_conn
		.in_transaction(async |conn| maps::set_state(map_id, state).exec(conn).await)
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

//=================================================================================================
// `/servers`

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateServerRequest
{
	name: ServerName,
	host: ServerHost,
	port: ServerPort,
	game: Game,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CreateServerResponse
{
	server_id: ServerId,
	access_key: AccessKey,
}

/// Register your KZ server
///
/// This endpoint can be used to register KZ servers with the API. If you are
/// a server owner, please make sure your server follows [the rules].
///
/// [the rules]: https://docs.cs2kz.org/servers/approval#rules
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	post,
	path = "/servers",
	tag = "Servers",
	security(("session_auth" = [])),
	request_body = CreateServerRequest,
	responses(
		(status = 201, body = CreateServerResponse),
		(status = 401,),
		(status = 409, body = ProblemDetails, description = "server properties conflict with existing server(s)"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn create_server(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Json(CreateServerRequest { name, host, port, game }): Json<CreateServerRequest>,
) -> HandlerResult<Created<CreateServerResponse>>
{
	if session.user_info().server_budget().is_exhausted() {
		debug!("server budget is exhausted");
		return Err(HandlerError::Unauthorized);
	}

	let (server_id, access_key) = database
		.in_transaction(async |db_conn| -> Result<_, CreateServerError> {
			let CreatedServer { id, access_key } = servers::create()
				.name(name)
				.host(host)
				.port(port)
				.game(game)
				.owned_by(session.user_info().id())
				.exec(&mut *db_conn)
				.await?;

			users::decrement_server_budget(session.user_info().id())
				.exec(db_conn)
				.await?;

			Ok((id, access_key))
		})
		.await?;

	Ok(Created(CreateServerResponse { server_id, access_key }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetServersQuery
{
	/// Only include servers with a matching name
	///
	/// If this parameter is specified, the returned servers will be ordered by
	/// how close their actual name matches the given value.
	name: Option<Box<str>>,

	/// Only include servers with a matching hostname / IP
	host: Option<Box<str>>,

	/// Only include servers for the specified game
	#[serde(default)]
	game: Game,

	/// Only include servers owned by the specified user
	owned_by: Option<UserId>,

	/// Include servers that currently don't have an API key
	#[serde(default)]
	include_degloballed: bool,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

/// Global KZ Servers
///
/// This endpoints returns KZ servers registered with the API. Servers which are
/// currently online and connected to the API will contain a `connection_info`
/// object with real-time information about the map they're currently hosting
/// and who's playing on them.
#[instrument(skip(database, server_monitor), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/servers",
	tag = "Servers",
	params(GetServersQuery),
	responses(
		(status = 200, body = PaginationResponse<Server>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_servers(
	State(database): State<database::ConnectionPool>,
	State(server_monitor): State<ServerMonitorHandle>,
	Query(GetServersQuery { name, host, game, owned_by, include_degloballed, offset, limit }): Query<GetServersQuery>,
) -> HandlerResult<Json<PaginationResponse<Server>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response = PaginationResponse::new({
		servers::count()
			.maybe_name(name.as_deref())
			.maybe_host(host.as_deref())
			.game(game)
			.maybe_owned_by(owned_by)
			.require_access_key(!include_degloballed)
			.exec(&mut db_conn)
			.await?
	});

	let mut servers = pin!({
		servers::get(game)
			.maybe_name(name.as_deref())
			.maybe_host(host.as_deref())
			.maybe_owned_by(owned_by)
			.require_access_key(!include_degloballed)
			.offset(offset.value())
			.limit(limit.value())
			.exec(&mut db_conn)
	});

	response.extend_reserve(servers.size_hint().0);
	while let Some(server) = servers.try_next().await? {
		response.extend_one(match server_monitor.get_connection_info(server.id).await {
			Ok(connection_info) => Server { connection_info: Some(connection_info), ..server },
			Err(_) => server,
		});
	}

	Ok(Json(response))
}

/// Global KZ Servers by ID
///
/// Returns a specific KZ server by its API-assigned ID.
#[instrument(skip(database, server_monitor), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/servers/{server_id}",
	tag = "Servers",
	params(("server_id" = ServerId, Path)),
	responses(
		(status = 200, body = Server),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_server(
	State(database): State<database::ConnectionPool>,
	State(server_monitor): State<ServerMonitorHandle>,
	Path(server_id): Path<ServerId>,
) -> HandlerResult<Json<Server>>
{
	let mut db_conn = database.acquire().await?;
	let mut server = servers::get_by_id(server_id)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	server.connection_info = server_monitor.get_connection_info(server.id).await.ok();

	Ok(Json(server))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateServerRequest
{
	name: Option<ServerName>,
	host: Option<ServerHost>,
	port: Option<ServerPort>,
	game: Option<Game>,
}

/// Update your KZ Server
///
/// This endpoint can be used by server owners to update the metadata of their
/// servers, such as IP/port.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	patch,
	path = "/servers/{server_id}",
	tag = "Servers",
	security(("session_auth" = ["modify-server-metadata"])),
	params(("server_id" = ServerId, Path)),
	request_body = UpdateServerRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 409, body = ProblemDetails, description = "server properties conflict with existing server(s)"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_server(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(server_id): Path<ServerId>,
	Json(UpdateServerRequest { name, host, port, game }): Json<UpdateServerRequest>,
) -> HandlerResult<NoContent>
{
	let mut db_conn = None;

	if !session
		.user_info()
		.permissions()
		.contains(&Permission::ModifyServerMetadata)
	{
		debug!("user does not have permissions");

		let owner_id = servers::get_owner_id(server_id)
			.exec(db_conn.insert(database.acquire().await?))
			.await?
			.ok_or(HandlerError::NotFound)?;

		if session.user_info().id() != owner_id {
			debug!("user is not the server owner");
			return Err(HandlerError::Unauthorized);
		}
	}

	let db_conn = match db_conn {
		Some(ref mut db_conn) => db_conn,
		None => db_conn.insert(database.acquire().await?),
	};

	let updated = db_conn
		.in_transaction(async |conn| {
			servers::update(server_id)
				.maybe_name(name)
				.maybe_host(host)
				.maybe_port(port)
				.maybe_game(game)
				.exec(conn)
				.await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ResetServerAccessKeyResponse
{
	access_key: AccessKey,
}

/// Generate a new access key
///
/// This endpoint can be used by server owners and admins to generate a new
/// access key for a server. This immediately invalidates the old key and cuts
/// off the server if it is currently connected to the API.
#[instrument(skip(database, server_monitor), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/servers/{server_id}/access-key",
	tag = "Servers",
	security(("session_auth" = ["reset-server-access-keys"])),
	params(("server_id" = ServerId, Path)),
	responses(
		(status = 201, body = ResetServerAccessKeyResponse),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
	),
)]
pub(crate) async fn reset_server_access_key(
	State(database): State<database::ConnectionPool>,
	State(server_monitor): State<ServerMonitorHandle>,
	session: auth::Session,
	Path(server_id): Path<ServerId>,
) -> HandlerResult<Created<ResetServerAccessKeyResponse>>
{
	if !session
		.user_info()
		.permissions()
		.contains(&Permission::ResetServerAccessKeys)
	{
		return Err(HandlerError::Unauthorized);
	}

	let access_key = database
		.in_transaction(async |db_conn| servers::reset_access_key(server_id).exec(db_conn).await)
		.await?
		.ok_or(HandlerError::NotFound)?;

	match server_monitor.disconnect_server(server_id).await {
		Ok(true) => {
			debug!("disconnected server");
		},
		Ok(false) => {
			debug!("did not disconnect server; not currently connected");
		},
		Err(server_monitor::DisconnectServerError::MonitorUnavailable) => {
			debug!("did not disconnect server; monitor unavailable");
		},
	}

	Ok(Created(ResetServerAccessKeyResponse { access_key }))
}

/// Delete a server's API key
///
/// This endpoint can be used by admins to delete a server's API key. This
/// immediately invalidates it and cuts off the server if it is currently
/// connected to the API.
#[instrument(skip(database, server_monitor), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	delete,
	path = "/servers/{server_id}/access-key",
	tag = "Servers",
	security(("session_auth" = ["delete-server-access-keys"])),
	params(("server_id" = ServerId, Path)),
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
	),
)]
pub(crate) async fn delete_server_access_key(
	State(database): State<database::ConnectionPool>,
	State(server_monitor): State<ServerMonitorHandle>,
	session: auth::Session,
	Path(server_id): Path<ServerId>,
) -> HandlerResult<NoContent>
{
	if !session
		.user_info()
		.permissions()
		.contains(&Permission::DeleteServerAccessKeys)
	{
		return Err(HandlerError::Unauthorized);
	}

	let servers_updated = database
		.in_transaction(async |db_conn| servers::delete_access_key([server_id]).exec(db_conn).await)
		.await?;

	match server_monitor.disconnect_server(server_id).await {
		Ok(true) => {
			debug!("disconnected server");
		},
		Ok(false) => {
			debug!("did not disconnect server; not currently connected");
		},
		Err(server_monitor::DisconnectServerError::MonitorUnavailable) => {
			debug!("did not disconnect server; monitor unavailable");
		},
	}

	if servers_updated > 0 { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

//=================================================================================================
// `/bans`

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateBanRequest
{
	player_id: PlayerId,
	reason: BanReason,
	expires_after: Option<Seconds>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CreateBanResponse
{
	ban_id: BanId,
	expires_at: Timestamp,
}

/// Ban a player
///
/// This endpoint can be used to restrict players from submitting records or
/// jumpstats to the API. Servers will also be informed about banned players
/// when they join.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	post,
	path = "/bans",
	tag = "Bans",
	security(("session_auth" = ["create-bans"])),
	request_body = CreateBanRequest,
	responses(
		(status = 201, body = CreateBanResponse),
		(status = 401,),
		(status = 409, body = ProblemDetails, description = "the player does not exist or is already banned"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn create_ban(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Json(CreateBanRequest { player_id, reason, expires_after }): Json<CreateBanRequest>,
) -> HandlerResult<Created<CreateBanResponse>>
{
	if !session.user_info().permissions().contains(&Permission::CreateBans) {
		debug!("user does not have permissions");
		return Err(HandlerError::Unauthorized);
	}

	let (ban_id, expires_at) = database
		.in_transaction(async |db_conn| -> Result<_, CreateBanError> {
			let CreatedBan { id, expires_at } = bans::create(player_id)
				.reason(reason)
				.banned_by(session.user_info().id())
				.maybe_expires_after(expires_after)
				.exec(&mut *db_conn)
				.await?;

			Ok((id, expires_at))
		})
		.await?;

	Ok(Created(CreateBanResponse { ban_id, expires_at }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetBansQuery
{
	/// Only include bans for this player
	player_id: Option<PlayerId>,

	/// Only include bans issued by this admin
	banned_by: Option<UserId>,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

/// Player Bans
///
/// This endpoint returns the latest player bans created by `POST /bans`.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/bans",
	tag = "Bans",
	params(GetBansQuery),
	responses(
		(status = 200, body = PaginationResponse<Ban>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_bans(
	State(database): State<database::ConnectionPool>,
	Query(GetBansQuery { player_id, banned_by, offset, limit }): Query<GetBansQuery>,
) -> HandlerResult<Json<PaginationResponse<Ban>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response = PaginationResponse::new({
		bans::count()
			.maybe_player(player_id)
			.maybe_banned_by(banned_by)
			.exec(&mut db_conn)
			.await?
	});

	bans::get()
		.maybe_player(player_id)
		.maybe_banned_by(banned_by)
		.offset(offset.value())
		.limit(limit.value())
		.exec(&mut db_conn)
		.try_collect_into(&mut response)
		.await?;

	Ok(Json(response))
}

/// Player Bans by ID
///
/// This endpoint returns information about a specific ban.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/bans/{ban_id}",
	tag = "Bans",
	params(("ban_id" = BanId, Path)),
	responses(
		(status = 200, body = Ban),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_ban(
	State(database): State<database::ConnectionPool>,
	Path(ban_id): Path<BanId>,
) -> HandlerResult<Json<Ban>>
{
	let mut db_conn = database.acquire().await?;
	let ban = bans::get_by_id(ban_id)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(ban))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateBanRequest
{
	reason: Option<BanReason>,
	expires_after: Option<Seconds>,
}

/// Update an existing Ban
///
/// This endpoint can be used to update the details of a ban, such as the ban
/// reason or duration.
///
/// **Do not use this endpoint to revert bans! Use `DELETE /bans/{ban_id}`
/// instead.**
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	patch,
	path = "/bans/{ban_id}",
	tag = "Bans",
	security(("session_auth" = ["update-bans"])),
	params(("ban_id" = BanId, Path)),
	request_body = UpdateBanRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_ban(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(ban_id): Path<BanId>,
	Json(UpdateBanRequest { reason, expires_after }): Json<UpdateBanRequest>,
) -> HandlerResult<NoContent>
{
	if !session.user_info().permissions().contains(&Permission::UpdateBans) {
		debug!("user does not have permissions");
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| {
			bans::update(ban_id)
				.maybe_reason(reason)
				.maybe_expires_after(expires_after)
				.exec(db_conn)
				.await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct RevertBanRequest
{
	reason: UnbanReason,
}

/// Revert a Ban
///
/// This endpoint can be used to revert a ban ("unban" a player). Only active
/// bans can be reverted and a player can only have one active ban at a time.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	delete,
	path = "/bans/{ban_id}",
	tag = "Bans",
	security(("session_auth" = ["revert-bans"])),
	params(("ban_id" = BanId, Path)),
	request_body = RevertBanRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 409, body = ProblemDetails, description = "the ban cannot be reverted"),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn revert_ban(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(ban_id): Path<BanId>,
	Json(RevertBanRequest { reason }): Json<RevertBanRequest>,
) -> HandlerResult<NoContent>
{
	if !session.user_info().permissions().contains(&Permission::RevertBans) {
		debug!("user does not have permissions");
		return Err(HandlerError::Unauthorized);
	}

	database
		.in_transaction(async |db_conn| {
			bans::revert(ban_id)
				.reason(reason)
				.unbanned_by(session.user_info().id())
				.exec(db_conn)
				.await
		})
		.await?;

	Ok(NoContent)
}

//=================================================================================================
// `/players`

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetPlayersQuery
{
	/// Only include players with a matching name
	///
	/// If this parameter is specified, the returned players will be ordered by
	/// how close their actual name matches the given value.
	name: Option<Box<str>>,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

/// KZ Players
///
/// This endpoint returns information about players who have joined KZ servers
/// before.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/players",
	tag = "Players",
	params(GetPlayersQuery),
	responses(
		(status = 200, body = PaginationResponse<Player>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_players(
	State(database): State<database::ConnectionPool>,
	Query(GetPlayersQuery { name, offset, limit }): Query<GetPlayersQuery>,
) -> HandlerResult<Json<PaginationResponse<Player>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response = PaginationResponse::new({
		players::count().maybe_name(name.as_deref()).exec(&mut db_conn).await?
	});

	players::get()
		.maybe_name(name.as_deref())
		.offset(offset.value())
		.limit(limit.value())
		.exec(&mut db_conn)
		.try_collect_into(&mut response)
		.await?;

	Ok(Json(response))
}

/// KZ Players by SteamID
///
/// This endpoint returns a specific player by their SteamID.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/players/{player_id}",
	tag = "Players",
	params(("player_id" = PlayerId, Path)),
	responses(
		(status = 200, body = Player),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_player(
	State(database): State<database::ConnectionPool>,
	Path(player_id): Path<PlayerId>,
) -> HandlerResult<Json<Player>>
{
	let mut db_conn = database.acquire().await?;
	let player = players::get_by_id(player_id)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(player))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetPlayerPreferencesQuery
{
	game: Game,
}

/// Player Preferences
///
/// This endpoint returns the in-game preferences of a specific player.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/players/{player_id}/preferences",
	tag = "Players",
	params(("player_id" = PlayerId, Path), GetPlayerPreferencesQuery),
	responses(
		(status = 200, body = PlayerPreferences),
		(status = 400, body = ProblemDetails, description = "invalid path/query parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_player_preferences(
	State(database): State<database::ConnectionPool>,
	Path(player_id): Path<PlayerId>,
	Query(GetPlayerPreferencesQuery { game }): Query<GetPlayerPreferencesQuery>,
) -> HandlerResult<Json<PlayerPreferences>>
{
	let mut db_conn = database.acquire().await?;
	let player = players::get_preferences(player_id)
		.game(game)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(player))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdatePlayerPreferencesRequest
{
	game: Game,
	preferences: PlayerPreferences,
}

/// Update Player Preferences
///
/// This endpoint can be used to update your in-game preferences without joining
/// a server and doing it manually there.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/players/{player_id}/preferences",
	tag = "Players",
	security(("session_auth" = [])),
	params(("player_id" = PlayerId, Path)),
	request_body = UpdatePlayerPreferencesRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_player_preferences(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(player_id): Path<PlayerId>,
	Json(UpdatePlayerPreferencesRequest { game, preferences }): Json<
		UpdatePlayerPreferencesRequest,
	>,
) -> HandlerResult<NoContent>
{
	if player_id != session.user_info().id() {
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| {
			players::update(player_id)
				.preferences((&preferences, game))
				.exec(db_conn)
				.await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
pub(crate) async fn recalculate_player_rating(
	State(database): State<database::ConnectionPool>,
	Path(player_id): Path<PlayerId>,
) -> HandlerResult<Json<PlayerRating>>
{
	let rating = database
		.in_transaction(async |db_conn| players::recalculate_rating(player_id).exec(db_conn).await)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(rating))
}

//=================================================================================================
// `/users`

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetUsersQuery
{
	/// Only include users that have permissions
	#[serde(default)]
	has_permissions: bool,

	/// Only include users with *at least* these permissions
	#[serde(default, rename = "permissions")]
	required_permissions: Permissions,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<100, 1000>,
}

/// API Users
///
/// This endpoint returns information about users that have logged into the API
/// before.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/users",
	tag = "Users",
	params(GetUsersQuery),
	responses(
		(status = 200, body = PaginationResponse<User>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_users(
	State(database): State<database::ConnectionPool>,
	Query(GetUsersQuery { has_permissions, required_permissions, offset, limit }): Query<
		GetUsersQuery,
	>,
) -> HandlerResult<Json<PaginationResponse<User>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response = PaginationResponse::new({
		users::count()
			.has_permissions(has_permissions)
			.required_permissions(required_permissions)
			.exec(&mut db_conn)
			.await?
	});

	users::get()
		.has_permissions(has_permissions)
		.required_permissions(required_permissions)
		.offset(offset.value())
		.limit(limit.value())
		.exec(&mut db_conn)
		.try_collect_into(&mut response)
		.await?;

	Ok(Json(response))
}

/// API Users by SteamID
///
/// This endpoint returns information about a specific user by their SteamID.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/users/{user_id}",
	tag = "Users",
	params(("user_id" = UserId, Path)),
	responses(
		(status = 200, body = User),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 404,),
	),
)]
pub(crate) async fn get_user(
	State(database): State<database::ConnectionPool>,
	Path(user_id): Path<UserId>,
) -> HandlerResult<Json<User>>
{
	let mut db_conn = database.acquire().await?;
	let user = users::get_by_id(user_id)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandlerError::NotFound)?;

	Ok(Json(user))
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateUserEmailRequest
{
	email: EmailAddress,
}

/// Update your Email address
///
/// This endpoint can be used to update your email address. The API will use
/// this for sending notifications, for example if you are a server owner.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/users/{user_id}/email",
	tag = "Users",
	security(("session_auth" = [])),
	params(("user_id" = UserId, Path)),
	request_body = UpdateUserEmailRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_user_email(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(user_id): Path<UserId>,
	Json(UpdateUserEmailRequest { email }): Json<UpdateUserEmailRequest>,
) -> HandlerResult<NoContent>
{
	if user_id != session.user_info().id() {
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| users::set_email(user_id, Some(email)).exec(db_conn).await)
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

/// Delete your Email address
///
/// This endpoint can be used to completely delete your email address from the
/// API. It will no longer be able to send you notifications anymore.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	delete,
	path = "/users/{user_id}/email",
	tag = "Users",
	security(("session_auth" = [])),
	params(("user_id" = UserId, Path)),
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
	),
)]
pub(crate) async fn delete_user_email(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(user_id): Path<UserId>,
) -> HandlerResult<NoContent>
{
	if user_id != session.user_info().id() {
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| users::set_email(user_id, None).exec(db_conn).await)
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateUserPermissionsRequest
{
	permissions: Permissions,
}

/// Update a user's permissions
///
/// This endpoint can be used to edit other users' permissions.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/users/{user_id}/permissions",
	tag = "Users",
	security(("session_auth" = ["modify-user-permissions"])),
	params(("user_id" = UserId, Path)),
	request_body = UpdateUserPermissionsRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_user_permissions(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(user_id): Path<UserId>,
	Json(UpdateUserPermissionsRequest { permissions }): Json<UpdateUserPermissionsRequest>,
) -> HandlerResult<NoContent>
{
	if !session
		.user_info()
		.permissions()
		.contains(&Permission::ModifyUserPermissions)
	{
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| {
			users::set_permissions(user_id, permissions).exec(db_conn).await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct UpdateServerBudgetRequest
{
	budget: ServerBudget,
}

/// Update a user's server budget
///
/// This endpoint can be used to set a user's server budget (how many servers
/// they are allowed to create).
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/users/{user_id}/server-budget",
	tag = "Users",
	security(("session_auth" = ["modify-server-budgets"])),
	params(("user_id" = UserId, Path)),
	request_body = UpdateServerBudgetRequest,
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn update_user_server_budget(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(user_id): Path<UserId>,
	Json(UpdateServerBudgetRequest { budget }): Json<UpdateServerBudgetRequest>,
) -> HandlerResult<NoContent>
{
	if !session
		.user_info()
		.permissions()
		.contains(&Permission::ModifyServerBudgets)
	{
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| {
			users::set_server_budget(user_id, budget).exec(db_conn).await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

//=================================================================================================
// `/mappers`

/// Mark a user as a mapper
///
/// This endpoint can be used to mark a user as a "mapper". This will allow them
/// to use the `PUT /maps` endpoint.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	put,
	path = "/mappers/{user_id}",
	tag = "Mappers",
	security(("session_auth" = ["grant-create-maps"])),
	params(("user_id" = UserId, Path)),
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
	),
)]
pub(crate) async fn create_mapper(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(user_id): Path<UserId>,
) -> HandlerResult<NoContent>
{
	if !session.user_info().permissions().contains(&Permission::GrantCreateMaps) {
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| {
			users::add_permissions(user_id, Permission::CreateMaps).exec(db_conn).await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

/// Mark a user as a non-mapper
///
/// This endpoint can be used to mark a user as not a "mapper". This will
/// prevent them from using the `PUT /maps` endpoint.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	delete,
	path = "/mappers/{user_id}",
	tag = "Mappers",
	security(("session_auth" = ["grant-create-maps"])),
	params(("user_id" = UserId, Path)),
	responses(
		(status = 204,),
		(status = 400, body = ProblemDetails, description = "invalid path parameter(s)"),
		(status = 401,),
		(status = 404,),
	),
)]
pub(crate) async fn delete_mapper(
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Path(user_id): Path<UserId>,
) -> HandlerResult<NoContent>
{
	if !session.user_info().permissions().contains(&Permission::GrantCreateMaps) {
		return Err(HandlerError::Unauthorized);
	}

	let updated = database
		.in_transaction(async |db_conn| {
			users::remove_permissions(user_id, Permission::CreateMaps)
				.exec(db_conn)
				.await
		})
		.await?;

	if updated { Ok(NoContent) } else { Err(HandlerError::NotFound) }
}

//=================================================================================================
// `/events`

/// Real-Time events
///
/// Returns an [SSE] response.
///
/// [SSE]: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events
#[instrument(skip(task_manager), ret(level = "debug"))]
#[utoipa::path(
	get,
	path = "/events",
	tag = "Events",
	responses(
		(status = 200, body = Event),
	),
)]
pub(crate) async fn events(State(task_manager): State<TaskManager>) -> Response
{
	#[pin_project]
	struct StreamState<S>
	{
		cancellation_token: CancellationToken,
		#[pin]
		events: S,
	}

	let state = StreamState {
		cancellation_token: task_manager.cancellation_token(),
		events: Box::pin(event_queue::subscribe()),
	};

	let stream = stream::unfold(state, async |mut state| {
		select! {
			() = state.cancellation_token.cancelled() => None,
			Some(event) = state.events.next() => {
				let event = sse::Event::default()
					.event(event.name())
					.json_data(event);

				Some((event, state))
			},
		}
	});

	Sse::new(stream.instrumented(tracing::info_span!("event_stream"))).into_response()
}

//=================================================================================================
// `/plugin`

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreatePluginVersionRequest
{
	/// A SemVer version
	version: PluginVersion,

	/// Which game this plugin is for
	game: Game,

	/// The git revision associated with the release commit / tag
	git_revision: GitRevision,

	/// Checksum of the plugin binary on Linux
	linux_checksum: Checksum,

	/// Checksum of the plugin binary on Windows
	windows_checksum: Checksum,

	/// Whether this release invalidates all previous releases
	is_cutoff: bool,

	modes: Vec<CreateModePluginVersionRequest>,

	styles: Vec<CreateStylePluginVersionRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateModePluginVersionRequest
{
	mode: Mode,
	linux_checksum: Checksum,
	windows_checksum: Checksum,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateStylePluginVersionRequest
{
	style: Style,
	linux_checksum: Checksum,
	windows_checksum: Checksum,
}

/// Register a new GOKZ/cs2kz-metamod version
///
/// This endpoint is used by GitHub CI to inform the API of new plugin versions.
#[instrument(skip(config, database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	post,
	path = "/plugin/versions",
	tag = "Plugin",
	security(("api_key" = [])),
	request_body = CreatePluginVersionRequest,
	responses(
		(status = 201,),
		(status = 401,),
		(status = 409, body = ProblemDetails,),
		(status = 422, body = ProblemDetails, description = "invalid request body"),
	),
)]
pub(crate) async fn create_plugin_version(
	State(config): State<Arc<Config>>,
	State(database): State<database::ConnectionPool>,
	Header(Authorization(bearer)): Header<Authorization<Bearer>>,
	Json(CreatePluginVersionRequest {
		version,
		game,
		git_revision,
		linux_checksum,
		windows_checksum,
		is_cutoff,
		modes,
		styles,
	}): Json<CreatePluginVersionRequest>,
) -> HandlerResult<Created<()>>
{
	let access_key = bearer.token().parse::<AccessKey>().map_err(|err| {
		debug!(error = &err as &dyn Error, "failed to parse access key");
		HandlerError::Unauthorized
	})?;

	let mut db_conn = database.acquire().await?;
	let access_key_name = access_key.fetch_name(&mut db_conn).await?.ok_or_else(|| {
		debug!("invalid access key");
		HandlerError::Unauthorized
	})?;

	if access_key_name != config.access_keys.cs2kz_metamod_release_key {
		debug!("wrong access key");
		return Err(HandlerError::Unauthorized);
	}

	db_conn
		.in_transaction(async |db_conn| {
			let modes = modes.into_iter().map(|req| {
				(req.mode, plugin::Checksums {
					linux: req.linux_checksum,
					windows: req.windows_checksum,
				})
			});

			let styles = styles.into_iter().map(|req| {
				(req.style, plugin::Checksums {
					linux: req.linux_checksum,
					windows: req.windows_checksum,
				})
			});

			plugin::create_version(version, game)
				.git_revision(git_revision)
				.linux_checksum(linux_checksum)
				.windows_checksum(windows_checksum)
				.is_cutoff(is_cutoff)
				.modes(modes)
				.styles(styles)
				.exec(&mut *db_conn)
				.await
		})
		.await?;

	Ok(Created(()))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GetPluginVersionsQuery
{
	/// Only include plugin versions for the specified game
	#[serde(default)]
	game: Game,

	/// Pagination offset
	#[serde(default)]
	offset: Offset,

	/// Limit the number of results returned
	#[serde(default)]
	limit: Limit<10, 1000>,
}

/// GOKZ/cs2kz-metamod versions
///
/// This endpoints returns metadata about official releases of the GOKZ and
/// cs2kz-metamod projects.
#[instrument(skip(database), ret(level = "debug"), err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/plugin/versions",
	tag = "Plugin",
	params(GetPluginVersionsQuery),
	responses(
		(status = 200, body = PaginationResponse<PluginVersionInfo>),
		(status = 400, body = ProblemDetails, description = "invalid query parameter(s)"),
	),
)]
pub(crate) async fn get_plugin_versions(
	State(database): State<database::ConnectionPool>,
	Query(GetPluginVersionsQuery { game, offset, limit }): Query<GetPluginVersionsQuery>,
) -> HandlerResult<Json<PaginationResponse<PluginVersionInfo>>>
{
	let mut db_conn = database.acquire().await?;
	let mut response =
		PaginationResponse::new(plugin::count_versions().game(game).exec(&mut db_conn).await?);

	plugin::get_versions()
		.game(game)
		.offset(offset.value())
		.limit(limit.value())
		.exec(&mut db_conn)
		.try_collect_into(&mut response)
		.await?;

	Ok(Json(response))
}

//=================================================================================================
// `/auth`

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CurrentSessionInfo
{
	created_at: Timestamp,
	expires_at: Timestamp,
}

/// Get information about your current session
#[instrument(ret(level = "debug"))]
#[utoipa::path(
	get,
	path = "/auth/web",
	tag = "User Authentication",
	responses(
		(status = 200, body = CurrentSessionInfo),
		(status = 401, description = "you are not logged in"),
	),
)]
pub(crate) async fn web_current_session(session: auth::Session) -> Json<CurrentSessionInfo>
{
	Json(CurrentSessionInfo { created_at: session.created_at(), expires_at: session.expires_at() })
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WebLoginRequest
{
	/// The URL to return to after the login process is complete.
	#[debug("{:?}", return_to.as_ref().map(|url| url.as_str()))]
	return_to: Option<Url>,
}

/// Login with Steam
///
/// This endpoint will redirect you to Steam for login.
#[instrument(skip(config), ret(level = "debug"))]
#[utoipa::path(
	get,
	path = "/auth/web/login",
	tag = "User Authentication",
	params(WebLoginRequest),
	responses(
		(status = 303, description = "redirect to Steam's login page"),
	),
)]
pub(crate) async fn web_login(
	State(config): State<Arc<Config>>,
	Query(WebLoginRequest { return_to }): Query<WebLoginRequest>,
) -> Redirect
{
	let userdata = return_to.as_ref().unwrap_or(&config.http.public_url);
	let return_to = config
		.http
		.public_url
		.join("/auth/web/__steam_callback")
		.unwrap_or_else(|err| panic!("failed to create OpenID `return_to` URL: {err}"));

	steam_openid::login_url(return_to, userdata)
		.map(|url| Redirect::to(url.as_str()))
		.unwrap_or_else(|err| panic!("failed to generate OpenID login URL: {err}"))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WebLogoutRequest
{
	/// Whether to invalidate all your currently active sessions
	#[serde(default)]
	invalidate_all: bool,
}

/// Logout
///
/// This endpoint can be used to delete your current, and optionally all other,
/// active session(s).
#[instrument(skip(config, database), ret(level = "debug"))]
#[utoipa::path(
	get,
	path = "/auth/web/logout",
	tag = "User Authentication",
	security(("session_auth" = [])),
	params(WebLogoutRequest),
	responses(
		(status = 204,),
		(status = 401, description = "you are not logged in"),
	),
)]
pub(crate) async fn web_logout(
	State(config): State<Arc<Config>>,
	State(database): State<database::ConnectionPool>,
	session: auth::Session,
	Query(WebLogoutRequest { invalidate_all }): Query<WebLogoutRequest>,
) -> HandlerResult<(CookieJar, NoContent)>
{
	session.invalidate();

	if invalidate_all {
		database
			.in_transaction(async |db_conn| {
				users::sessions::expire_active(session.user_info().id())
					.exec(db_conn)
					.await
			})
			.await?;
	}

	let player_cookie = config
		.http
		.cookies
		.cookie_builder(PLAYER_COOKIE_NAME, "")
		.removal()
		.build();

	Ok((CookieJar::default().add(player_cookie), NoContent))
}

#[instrument(
	skip(config, database, steam_api_client),
	ret(level = "debug"),
	err(Debug, level = "debug")
)]
pub(crate) async fn steam_auth_callback(
	State(config): State<Arc<Config>>,
	State(database): State<database::ConnectionPool>,
	State(steam_api_client): State<steam::api::Client>,
	Query(payload): Query<steam_openid::CallbackPayload>,
) -> HandlerResult<(CookieJar, Redirect)>
{
	let expected_host = config
		.http
		.public_url
		.host()
		.unwrap_or_else(|| panic!("`http.public-url` should have a host part"));

	let send_request = async |req| {
		let req = reqwest::Request::try_from(req).unwrap_or_else(|err| {
			panic!("hard-coded request should be valid: {err}");
		});

		steam_api_client
			.as_ref()
			.execute(req)
			.map_ok(http::Response::<reqwest::Body>::from)
			.await
	};

	match payload.clone().verify(expected_host, send_request).await {
		Ok(user_id) => {
			let Some(steam_user) = steam::users::get(&steam_api_client, user_id).await? else {
				warn!(%user_id, "user logged in successfully but failed to fetch info");
				return Err(HandlerError::Unauthorized);
			};

			assert_eq!(steam_user.id, user_id);

			let user_id = UserId::from(steam_user.id);
			let username = steam_user
				.name
				.parse::<Username>()
				.inspect_err_dyn(|error| warn!(error, "steam user has invalid username"))
				.map_err(|_| HandlerError::Unauthorized)?;

			let session_id = database
				.in_transaction(async |db_conn| {
					users::create(user_id).name(username).exec(&mut *db_conn).await?;
					users::sessions::create(user_id)
						.expires_after(config.http.cookies.max_age_auth)
						.exec(db_conn)
						.await
				})
				.await?;

			let user_json = serde_json::to_string(&steam_user).map_err(|err| {
				error!(error = &err as &dyn Error, "failed to serialize JSON");
				HandlerError::Internal
			})?;

			let player_cookie =
				config.http.cookies.cookie_builder(PLAYER_COOKIE_NAME, user_json).build();

			let session_cookie = config
				.http
				.cookies
				.auth_cookie_builder(SessionId::COOKIE_NAME, session_id.to_string())
				.build();

			let cookies = CookieJar::default().add(player_cookie).add(session_cookie);
			let redirect = Redirect::to(&payload.userdata);

			Ok((cookies, redirect))
		},
		Err(error) => match *error.kind() {
			VerifyCallbackPayloadErrorKind::HostMismatch => {
				debug!("login failed due to hostname mismatch");
				Err(HandlerError::Unauthorized)
			},
			VerifyCallbackPayloadErrorKind::HttpRequest(ref error) => {
				debug!(error = error as &dyn Error);
				Err(HandlerError::Problem(ProblemDetails::new(ProblemType::SteamApiError)))
			},
			VerifyCallbackPayloadErrorKind::BadStatus { ref response } => {
				debug!(
					res.status = response.status().as_u16(),
					res.body = str::from_utf8(response.body()).unwrap_or("<invalid utf-8>"),
					"bad status",
				);

				Err(HandlerError::Unauthorized)
			},
			VerifyCallbackPayloadErrorKind::BufferResponseBody { ref error, ref response } => {
				error!(error = error as &dyn Error, res.status = response.status.as_u16());
				Err(HandlerError::Unauthorized)
			},
			VerifyCallbackPayloadErrorKind::InvalidPayload { ref response } => {
				debug!(
					res.status = response.status().as_u16(),
					res.body = str::from_utf8(response.body()).unwrap_or("<invalid utf-8>"),
					"bad payload",
				);

				Err(HandlerError::Unauthorized)
			},
		},
	}
}

#[instrument(skip(database, server_monitor), ret(level = "debug"))]
pub(crate) async fn cs2_auth(
	State(database): State<database::ConnectionPool>,
	State(server_monitor): State<ServerMonitorHandle>,
	Header(Authorization(bearer)): Header<Authorization<Bearer>>,
	upgrade: WebSocketUpgrade,
) -> HandlerResult<Response>
{
	let access_key = bearer.token().parse::<AccessKey>().map_err(|err| {
		debug!(error = &err as &dyn Error, "failed to parse access key");
		HandlerError::Unauthorized
	})?;

	let server_id = {
		let mut db_conn = database.acquire().await?;
		servers::get_id_by_access_key(access_key)
			.exec(&mut db_conn)
			.await?
			.ok_or(HandlerError::Unauthorized)?
	};

	server_monitor
		.server_connecting(server_id, upgrade)
		.await
		.map_err(|err| match err {
			server_monitor::ServerConnectingError::MonitorUnavailable => HandlerError::ShuttingDown,
			server_monitor::ServerConnectingError::AlreadyConnected => HandlerError::Unauthorized,
		})
}
