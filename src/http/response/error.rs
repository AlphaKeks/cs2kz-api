use std::{error::Error, panic::Location};

use axum::response::{IntoResponse, Response};
use cs2kz_api::{
	bans::{CreateBanError, RevertBanError, UpdateBanError},
	database::DatabaseError,
	maps::{CreateMapError, MapState, UpdateMapError},
	plugin::CreatePluginVersionError,
	servers::{CreateServerError, UpdateServerError},
	steam,
};

use crate::http::problem_details::{ProblemDetails, ProblemType};

pub(crate) type HandlerResult<T> = Result<T, HandlerError>;

#[derive(Debug)]
pub(crate) enum HandlerError
{
	Unauthorized,
	NotFound,
	Internal,
	ShuttingDown,

	#[debug("Problem({:?})", _0.problem_type())]
	Problem(ProblemDetails),
}

impl IntoResponse for HandlerError
{
	fn into_response(self) -> Response
	{
		match self {
			HandlerError::Unauthorized => http::StatusCode::UNAUTHORIZED.into_response(),
			HandlerError::NotFound => http::StatusCode::NOT_FOUND.into_response(),
			HandlerError::Internal => http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
			HandlerError::ShuttingDown => http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
			HandlerError::Problem(problem_details) => problem_details.into_response(),
		}
	}
}

impl From<ProblemDetails> for HandlerError
{
	fn from(problem_details: ProblemDetails) -> Self
	{
		Self::Problem(problem_details)
	}
}

impl From<DatabaseError> for HandlerError
{
	#[track_caller]
	fn from(error: DatabaseError) -> Self
	{
		tracing::error!(loc = %Location::caller(), error = &error as &dyn Error);
		Self::Internal
	}
}

impl From<steam::ApiError> for HandlerError
{
	fn from(error: steam::ApiError) -> Self
	{
		match error {
			steam::ApiError::Http(ref error) => {
				if error.status().is_some_and(|status| status.is_client_error()) {
					tracing::debug!(error = error as &dyn Error);
					HandlerError::Internal
				} else {
					ProblemDetails::new(ProblemType::SteamApiError).into()
				}
			},
			steam::ApiError::BufferResponseBody { ref error, ref response } => {
				tracing::error!(error = error as &dyn Error, res.status = response.status.as_u16());
				ProblemDetails::new(ProblemType::SteamApiError).into()
			},
			steam::ApiError::DeserializeResponse { ref error, ref response } => {
				tracing::debug!(
					res.status = response.status().as_u16(),
					res.body = str::from_utf8(response.body()).unwrap_or("<invalid utf-8>"),
					error = error as &dyn Error,
					"bad payload",
				);

				ProblemDetails::new(ProblemType::SteamApiError).into()
			},
		}
	}
}

impl From<CreateServerError> for HandlerError
{
	fn from(error: CreateServerError) -> Self
	{
		match error {
			CreateServerError::NameAlreadyInUse => {
				ProblemDetails::new(ProblemType::ServerNameAlreadyInUse).into()
			},
			CreateServerError::HostAndPortAlreadyInUse => {
				ProblemDetails::new(ProblemType::ServerHostAndPortAlreadyInUse).into()
			},
			CreateServerError::DatabaseError(database_error) => database_error.into(),
		}
	}
}

impl From<UpdateServerError> for HandlerError
{
	fn from(error: UpdateServerError) -> Self
	{
		match error {
			UpdateServerError::NameAlreadyInUse => {
				ProblemDetails::new(ProblemType::ServerNameAlreadyInUse).into()
			},
			UpdateServerError::HostAndPortAlreadyInUse => {
				ProblemDetails::new(ProblemType::ServerHostAndPortAlreadyInUse).into()
			},
			UpdateServerError::DatabaseError(database_error) => database_error.into(),
		}
	}
}

impl From<CreateMapError> for HandlerError
{
	fn from(error: CreateMapError) -> Self
	{
		match error {
			CreateMapError::InvalidMapperId(user_id) => {
				let mut problem_details = ProblemDetails::new(ProblemType::InvalidMapperId);
				problem_details.add_extension_member("invalid_mapper_id", &user_id);
				problem_details.into()
			},
			CreateMapError::InvalidMapperName { id, error } => {
				let mut problem_details = ProblemDetails::new(ProblemType::InvalidMapperName);
				problem_details.set_detail(error.to_string());
				problem_details.add_extension_member("invalid_mapper_id", &id);
				problem_details.into()
			},
			CreateMapError::SteamApiError(api_error) => api_error.into(),
			CreateMapError::MapIsFrozen { id, state } => {
				let mut problem_details = ProblemDetails::new(ProblemType::MapIsFrozen);
				problem_details.add_extension_member("map_id", &id);
				problem_details.add_extension_member("map_state", &state);
				problem_details.set_detail(match state {
					MapState::Graveyard | MapState::WIP => unreachable!(),
					MapState::Pending => {
						"you already submitted the map for approval and have to wait for a \
						 decision before you can update it again"
					},
					MapState::Approved => "your map has already been approved",
					MapState::Completed => "you have already marked your map as 'completed'",
				});

				problem_details.into()
			},
			CreateMapError::NotTheMapper => Self::Unauthorized,
			CreateMapError::Database(database_error) => database_error.into(),
		}
	}
}

impl From<UpdateMapError> for HandlerError
{
	fn from(error: UpdateMapError) -> Self
	{
		match error {
			UpdateMapError::InvalidMapId => Self::NotFound,
			UpdateMapError::InvalidCourseLocalId(course_local_id) => {
				let mut problem_details = ProblemDetails::new(ProblemType::InvalidCourseId);
				problem_details.add_extension_member("course_local_id", &course_local_id);
				problem_details.into()
			},
			UpdateMapError::DatabaseError(database_error) => database_error.into(),
		}
	}
}

impl From<CreateBanError> for HandlerError
{
	fn from(error: CreateBanError) -> Self
	{
		match error {
			CreateBanError::UnknownPlayer => {
				ProblemDetails::new(ProblemType::UnknownPlayerToBan).into()
			},
			CreateBanError::AlreadyBanned => {
				ProblemDetails::new(ProblemType::PlayerAlreadyBanned).into()
			},
			CreateBanError::DatabaseError(database_error) => database_error.into(),
		}
	}
}

impl From<UpdateBanError> for HandlerError
{
	fn from(error: UpdateBanError) -> Self
	{
		match error {
			UpdateBanError::ExpiresInThePast => {
				ProblemDetails::new(ProblemType::BanExpiresInThePast).into()
			},
			UpdateBanError::DatabaseError(database_error) => database_error.into(),
		}
	}
}

impl From<RevertBanError> for HandlerError
{
	fn from(error: RevertBanError) -> Self
	{
		match error {
			RevertBanError::AlreadyExpired => {
				ProblemDetails::new(ProblemType::BanAlreadyExpired).into()
			},
			RevertBanError::AlreadyUnbanned => {
				ProblemDetails::new(ProblemType::BanAlreadyReverted).into()
			},
			RevertBanError::InvalidBanId => Self::NotFound,
			RevertBanError::Database(database_error) => database_error.into(),
		}
	}
}

impl From<CreatePluginVersionError> for HandlerError
{
	fn from(error: CreatePluginVersionError) -> Self
	{
		match error {
			CreatePluginVersionError::VersionAlreadyExists => {
				ProblemDetails::new(ProblemType::PluginVersionAlreadyExists).into()
			},
			CreatePluginVersionError::VersionOlderThanLatest { latest } => {
				let mut problem_details =
					ProblemDetails::new(ProblemType::PluginVersionIsOlderThanLatest);

				problem_details.add_extension_member("latest_version", &latest);
				problem_details.into()
			},
			CreatePluginVersionError::DatabaseError(database_error) => database_error.into(),
		}
	}
}
