//! Errors that can occur during HTTP requests, usually in handler functions.

use std::error::Error as StdError;
use std::fmt::Display;
use std::panic::Location;

use axum::response::{IntoResponse, Response};
use axum::Json;
use cs2kz::Mode;
use reqwest::StatusCode;
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error};

use crate::maps::{CourseID, MapID};

/// Return type for fallible HTTP handler functions.
pub type HandlerResult<T> = Result<T, HandlerError>;

/// The main error type returned by HTTP handlers.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct HandlerError {
	/// HTTP status code to return.
	status: StatusCode,

	/// Error message to include in the response.
	message: String,

	/// Location of where the error was created.
	location: &'static Location<'static>,

	/// A source error.
	source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl HandlerError {
	/// Creates a new [`HandlerError`] with the given `status` code and a default
	/// error message.
	#[track_caller]
	fn new(status: StatusCode) -> Self {
		Self {
			status,
			message: String::from("something went wrong"),
			location: Location::caller(),
			source: None,
		}
	}

	/// Set an error message.
	pub fn with_message<M>(mut self, message: M) -> Self
	where
		M: Display,
	{
		self.message = message.to_string();
		self
	}

	/// Run a closure, e.g. to log out the error.
	fn inspect<F>(self, f: F) -> Self
	where
		F: FnOnce(&Location<'_>),
	{
		f(self.location);
		self
	}

	/// Set a source error.
	pub fn with_source<S>(mut self, source: S) -> Self
	where
		S: StdError + Send + Sync + 'static,
	{
		self.source = Some(Box::new(source));
		self
	}

	/// The error to return when there is no data to return.
	#[track_caller]
	pub fn no_content() -> Self {
		Self::new(StatusCode::NO_CONTENT).with_message("")
	}

	/// The error to return when user input is somehow invalid.
	#[track_caller]
	pub fn bad_request() -> Self {
		Self::new(StatusCode::BAD_REQUEST)
	}

	/// The error to return when user input like an ID was technically valid, but not present
	/// in the database.
	#[track_caller]
	pub fn unknown<T>(what: T) -> Self
	where
		T: Display,
	{
		Self::bad_request().with_message(format!("unknown {what}"))
	}

	/// The error to return when authentication / authorization fails.
	#[track_caller]
	pub fn unauthorized() -> Self {
		Self::new(StatusCode::UNAUTHORIZED)
	}

	/// The error to return if the creation of a resource failed, because the resource already
	/// exists.
	#[track_caller]
	pub fn already_exists<T>(what: T) -> Self
	where
		T: Display,
	{
		Self::new(StatusCode::CONFLICT).with_message(format!("{what} already exists"))
	}

	/// When deleting mappers for a map, the user shouldn't be able to delete all of them, as
	/// a map must have at least 1 mapper.
	#[track_caller]
	pub fn map_must_have_mappers() -> Self {
		Self::new(StatusCode::CONFLICT)
			.with_message("can't delete all mappers (maps must have at least 1 mapper)")
	}

	/// When deleting mappers for a course, the user shouldn't be able to delete all of them,
	/// as a course must have at least 1 mapper.
	#[track_caller]
	pub fn course_must_have_mappers(course_id: CourseID) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format!(
			"can't delete all mappers of course `{course_id}` (courses must have at least 1 mapper)",
		))
	}

	/// User submitted an update for a map, with a course ID of a course that does not belong
	/// to that map.
	#[track_caller]
	pub fn course_does_not_belong_to_map(course_id: CourseID, map_id: MapID) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format!(
			"course `{course_id}` does not belong to map `{map_id}`"
		))
	}

	/// User submitted multiple updates for the same course filter.
	#[track_caller]
	pub fn duplicate_filter(mode: Mode, teleports: bool) -> Self {
		Self::new(StatusCode::CONFLICT).with_message(format!(
			"duplicate filter ({mode}, {runtype})",
			runtype = if teleports { "TP" } else { "Pro" },
		))
	}

	/// Something went wrong.
	///
	/// Returning this error indicates a bug.
	#[track_caller]
	pub fn internal_server_error() -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR)
			.inspect(|location| error!(target: "audit_log", %location, "internal server error"))
	}

	/// Something went wrong when making an external API call.
	#[track_caller]
	pub fn bad_gateway() -> Self {
		Self::new(StatusCode::BAD_GATEWAY)
			.inspect(|location| error!(target: "audit_log", %location, "external API call failed"))
	}
}

impl IntoResponse for HandlerError {
	#[allow(clippy::indexing_slicing)]
	fn into_response(self) -> Response {
		let Self {
			status,
			message,
			location,
			..
		} = &self;

		debug!(%location, %status, ?message, "error occurred in request handler");

		let mut json = json!({ "message": self.to_string() });

		if let Some(source) = self
			.source
			.as_deref()
			.filter(|_| cfg!(not(feature = "production")))
		{
			json["debug_info"] = format!("{source:?}").into();
		}

		(self.status, Json(json)).into_response()
	}
}

impl From<sqlx::Error> for HandlerError {
	#[track_caller]
	fn from(error: sqlx::Error) -> Self {
		use sqlx::Error as E;

		match error {
			E::Configuration(_) | E::Tls(_) | E::AnyDriverError(_) | E::Migrate(_) => {
				unreachable!("these do not happen after initial setup ({error})");
			}
			error => Self::internal_server_error()
				.with_message("database error")
				.inspect(|location| {
					error!(target: "audit_log", %error, %location, "database error");
				})
				.with_source(error),
		}
	}
}

impl From<jwt::errors::Error> for HandlerError {
	#[track_caller]
	fn from(error: jwt::errors::Error) -> Self {
		Self::internal_server_error()
			.with_message("failed to encode jwt")
			.with_source(error)
	}
}
