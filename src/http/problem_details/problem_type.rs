use {::problem_details::ProblemType as _, serde::Serialize, std::fmt};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProblemType
{
	InvalidPathParameters,
	InvalidQueryParameters,
	MissingHeader,
	InvalidHeader,
	DeserializeRequestBody,

	ServerNameAlreadyInUse,
	ServerHostAndPortAlreadyInUse,

	InvalidWorkshopId,
	InvalidMapName,
	InvalidMapperId,
	InvalidMapperName,
	InvalidCourseId,
	MapIsFrozen,
	InvalidFilterForGame,

	UnknownPlayerToBan,
	PlayerAlreadyBanned,
	BanExpiresInThePast,
	BanAlreadyExpired,
	BanAlreadyReverted,

	PluginVersionAlreadyExists,
	PluginVersionIsOlderThanLatest,

	SteamApiError,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct ProblemDescription
{
	problem_type: ProblemType,
	#[serde(serialize_with = "cs2kz_api::serde::ser::http::status_code")]
	status: http::StatusCode,
	description: &'static str,
}

impl ProblemType
{
	pub(crate) const ALL: &[Self] = &[
		Self::InvalidPathParameters,
		Self::InvalidQueryParameters,
		Self::MissingHeader,
		Self::InvalidHeader,
		Self::DeserializeRequestBody,
		Self::ServerNameAlreadyInUse,
		Self::ServerHostAndPortAlreadyInUse,
		Self::InvalidWorkshopId,
		Self::InvalidMapName,
		Self::InvalidMapperId,
		Self::InvalidMapperName,
		Self::InvalidCourseId,
		Self::MapIsFrozen,
		Self::InvalidFilterForGame,
		Self::UnknownPlayerToBan,
		Self::PlayerAlreadyBanned,
		Self::BanExpiresInThePast,
		Self::BanAlreadyExpired,
		Self::BanAlreadyReverted,
		Self::PluginVersionAlreadyExists,
		Self::PluginVersionIsOlderThanLatest,
		Self::SteamApiError,
	];

	pub(crate) fn description(&self) -> ProblemDescription
	{
		let description = match *self {
			Self::InvalidPathParameters => {
				"You supplied path parameters that could not be parsed correctly. The `detail` \
				 field of the response body should indicate what you did wrong."
			},
			Self::InvalidQueryParameters => {
				"You provided a URI query parameter with a value that could not be parsed \
				 correctly. The `detail` field of the response body should indicate what you did \
				 wrong."
			},
			Self::MissingHeader => {
				"You did not provide a required header. Some endpoints require certain headers to \
				 be provided, like `Content-Type: application/json` when sending a JSON request \
				 body."
			},
			Self::InvalidHeader => {
				"You provided a header with a value could not be parsed correctly. The `detail` \
				 field of the response body should indicate what you did wrong."
			},
			Self::DeserializeRequestBody => {
				"You provided a request body that could not be parsed correctly. The `detail` \
				 field of the response body should indicate what you did wrong."
			},
			Self::ServerNameAlreadyInUse => {
				"You tried to create or update a server and chose a `name` that is already used by \
				 another server. Server names must be unique, so double check if there's already \
				 another server with the name you chose."
			},
			Self::ServerHostAndPortAlreadyInUse => {
				"You tried to create or update a server and chose a combination of `host` and \
				 `port` values that are already used by another server. Combinations of `host` and \
				 `port` must be unique, so double check if there's already another server with the \
				 `host` / `port` combination you chose."
			},
			Self::InvalidWorkshopId => {
				"You tried to create a map and specified a `workshop_id` which the API could not \
				 use to fetch map details from Steam's API. Double check if you copied the right \
				 ID. In the URL `https://steamcommunity.com/sharedfiles/filedetails/?id=3121168339` \
				 the ID you want is `3121168339`."
			},
			Self::InvalidMapName => {
				"You tried to create a map with a name that does not conform to \
				 [the map approval rules](http://docs.cs2kz.org/mapping/approval#rules). Make sure \
				 it starts with `kz_`, only uses allowed characters, and does not exceed 27 \
				 characters in length."
			},
			Self::InvalidMapperId => {
				"You tried to create or update a map and specified a mapper's SteamID which the \
				 API could not use to fetch user details from Steam's API. Double check you got \
				 the right SteamID. If you used the `STEAM_X:Y:Z` format and `X` is `0`, try `1` \
				 instead. Many websites will incorrectly show `0` for X for IDs that should have \
				 a `1` there."
			},
			Self::InvalidMapperName => {
				"You tried to create or update a map and specified the SteamID of an account that \
				 has an invalid name. This shouldn't happen in practice, as the API places no \
				 requirements on user/player names other than 'not empty' (which Steam should also
				 enforce already), but you never know."
			},
			Self::InvalidCourseId => {
				"You tried to update a map and specified a course ID the API doesn't know about. \
				 The ID is chosen by you, in Hammer, so double check you got the right one. You \
				 cannot add or remove courses by updating an existing map, so if you modified your \
				 map in that way, submit a new one instead (which will invalidate the old one)."
			},
			Self::MapIsFrozen => {
				"You tried to update your map while it was in a 'frozen' state. Depending on which \
				 stage your map is in, you may not be allowed to make changes. For example, after \
				 submitting your map for approval, you have to wait for the Map Approval Team to \
				 make a decision and either approve or reject your map. If it is rejected, you are \
				 allowed to make further changes. The `map_state` field of the response body \
				 should tell you which state your map is currently in."
			},
			Self::InvalidFilterForGame => {
				"You tried to create a new map with filters that don't match the game the map is \
				 for. If you are creating a map for CS2, make sure the filters on all courses are \
				 also for CS2, and vice-versa for CSGO."
			},
			Self::UnknownPlayerToBan => {
				"You tried to ban a player the API doesn't know about."
			},
			Self::PlayerAlreadyBanned => {
				"You tried to ban a player who is already banned."
			},
			Self::BanExpiresInThePast => {
				"You tried to update a ban's duration in such a way that the new expiration date \
				 would be in the past. If you intended to revert the ban, use \
				 `DELETE /bans/{ban_id}` instead."
			},
			Self::BanAlreadyExpired => {
				"You tried to revert a ban that had already naturally expired."
			},
			Self::BanAlreadyReverted => {
				"You tried to revert a ban that had already been manually reverted."
			},
			Self::PluginVersionAlreadyExists => {
				"You tried to submit a new plugin version that the API already knew about."
			},
			Self::PluginVersionIsOlderThanLatest => {
				"You tried to submit a new plugin version that is older than the latest version \
				 the API knows about."
			},
			Self::SteamApiError => {
				"Steam's API returned an error. This usually means there are problems on Steam's \
				 side, so you may first try to wait a bit, and make a bug report if the issue \
				 persists."
			}
		};

		ProblemDescription { problem_type: *self, status: self.status(), description }
	}
}

macro uri($problem:literal) {
	http::Uri::from_static(concat!("https://docs.cs2kz.org/api/problems/", $problem))
}

impl ::problem_details::ProblemType for ProblemType
{
	fn uri(&self) -> http::Uri
	{
		match *self {
			Self::InvalidPathParameters => uri!("invalid-path-parameters"),
			Self::InvalidQueryParameters => uri!("invalid-query-parameters"),
			Self::MissingHeader => uri!("missing-header"),
			Self::InvalidHeader => uri!("invalid-header"),
			Self::DeserializeRequestBody => uri!("deserialize-request-body"),
			Self::ServerNameAlreadyInUse => uri!("server-name-already-in-use"),
			Self::ServerHostAndPortAlreadyInUse => {
				uri!("server-host-and-port-already-in-use")
			},
			Self::InvalidWorkshopId => uri!("invalid-map-id"),
			Self::InvalidMapName => uri!("invalid-map-name"),
			Self::InvalidMapperId => uri!("invalid-mapper-id"),
			Self::InvalidMapperName => uri!("invalid-mapper-name"),
			Self::InvalidCourseId => uri!("invalid-course-id"),
			Self::MapIsFrozen => uri!("map-is-frozen"),
			Self::InvalidFilterForGame => uri!("invalid-filter-for-game"),
			Self::UnknownPlayerToBan => uri!("unknown-player-to-ban"),
			Self::PlayerAlreadyBanned => uri!("player-already-banned"),
			Self::BanExpiresInThePast => uri!("ban-expires-in-the-past"),
			Self::BanAlreadyExpired => uri!("ban-already-expired"),
			Self::BanAlreadyReverted => uri!("ban-already-reverted"),
			Self::PluginVersionAlreadyExists => uri!("plugin-version-already-exists"),
			Self::PluginVersionIsOlderThanLatest => uri!("plugin-version-is-older-than-latest"),
			Self::SteamApiError => uri!("steam-api-error"),
		}
	}

	fn status(&self) -> http::StatusCode
	{
		match *self {
			Self::InvalidPathParameters
			| Self::InvalidQueryParameters
			| Self::MissingHeader
			| Self::InvalidHeader => http::StatusCode::BAD_REQUEST,
			Self::MapIsFrozen => http::StatusCode::FORBIDDEN,
			Self::InvalidWorkshopId
			| Self::InvalidMapName
			| Self::InvalidMapperId
			| Self::InvalidMapperName
			| Self::InvalidCourseId
			| Self::InvalidFilterForGame
			| Self::ServerNameAlreadyInUse
			| Self::ServerHostAndPortAlreadyInUse
			| Self::UnknownPlayerToBan
			| Self::PlayerAlreadyBanned
			| Self::BanExpiresInThePast
			| Self::BanAlreadyExpired
			| Self::BanAlreadyReverted
			| Self::PluginVersionAlreadyExists
			| Self::PluginVersionIsOlderThanLatest => http::StatusCode::CONFLICT,
			Self::DeserializeRequestBody => http::StatusCode::UNPROCESSABLE_ENTITY,
			Self::SteamApiError => http::StatusCode::BAD_GATEWAY,
		}
	}

	fn title(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.write_str(match *self {
			Self::InvalidPathParameters => "invalid path parameter(s)",
			Self::InvalidQueryParameters => "invalid query parameters",
			Self::MissingHeader => "missing required header",
			Self::InvalidHeader => "invalid header value",
			Self::DeserializeRequestBody => "failed to deserialize request body",
			Self::ServerNameAlreadyInUse => "server name is already in use",
			Self::ServerHostAndPortAlreadyInUse => "server host and port already in use",
			Self::InvalidWorkshopId => "invalid workshop ID",
			Self::InvalidMapName => "invalid map name",
			Self::InvalidMapperId => "invalid mapper ID",
			Self::InvalidMapperName => "invalid mapper name",
			Self::InvalidCourseId => "invalid course ID",
			Self::MapIsFrozen => "you may not update the map in its current state",
			Self::InvalidFilterForGame => "filters don't match target game",
			Self::UnknownPlayerToBan => "unknown player",
			Self::PlayerAlreadyBanned => "player is already banned",
			Self::BanExpiresInThePast => "ban would expire in the past",
			Self::BanAlreadyExpired => "ban has already expired",
			Self::BanAlreadyReverted => "ban has already been reverted",
			Self::PluginVersionAlreadyExists => "plugin version already exists",
			Self::PluginVersionIsOlderThanLatest => {
				"plugin version is older than the latest version"
			},
			Self::SteamApiError => "steam api error",
		})
	}
}

impl utoipa::PartialSchema for ProblemType
{
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>
	{
		use utoipa::openapi::schema::{self, Object};

		let enum_values = ProblemType::ALL
			.iter()
			.map(|problem| ::problem_details::ProblemType::uri(problem))
			.map(|uri| uri.to_string());

		let example =
			::problem_details::ProblemType::uri(&ProblemType::DeserializeRequestBody).to_string();

		Object::builder()
			.schema_type(schema::Type::String)
			.enum_values(Some(enum_values))
			.examples([example])
			.into()
	}
}
