//! The main error type.
//!
//! This is returned by all fallible HTTP handlers, middlewares, etc.

use std::fmt;
use std::panic::Location;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use itertools::Itertools;
use serde::Serialize;

/// Type alias that defaults to our [`Error`] as the default error type, but is
/// still overridable and therefore compatible with [`std::result::Result`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Convenience type alias.
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The main runtime error type.
///
/// This is the only error type allowed to reach users!
pub struct Error
{
	/// We box this so our error type is only 1 pointer wide.
	inner: Box<Inner>,
}

/// The different kinds of errors that can occur at runtime.
#[derive(Debug, thiserror::Error)]
enum ErrorKind
{
	/// We have no data to return.
	#[error("no content")]
	NoContent,

	/// Request was somehow malformed.
	#[error(transparent)]
	BadRequest(BoxError),

	/// Request could not be authenticated / authorized.
	#[error("you are not permitted to perform this operation")]
	Unauthorized(BoxError),

	/// A resource does not exist.
	#[error("{what} not found")]
	NotFound
	{
		/// The thing that could not be found.
		what: String,
	},

	/// Something went wrong communicating with the database.
	#[error("database error; please report this incident")]
	Database(#[from] sqlx::Error),

	/// An HTTP handler panicked, but was caught by middleware.
	#[error("something unexpected happened; please report this incident")]
	Panic,
}

impl Error
{
	/// Create a new [`Error`].
	#[track_caller]
	fn new(kind: ErrorKind) -> Self
	{
		Self { inner: Box::new(Inner::new(kind)) }
	}

	/// Returns the original error source.
	pub fn source(&self) -> &(dyn std::error::Error + Send + Sync + 'static)
	{
		&self.inner.kind
	}

	/// Returns the source code location of the original error source.
	pub fn source_location(&self) -> Location<'static>
	{
		self.inner.source_location
	}

	/// Returns the list of attachments on this error.
	pub fn attachments(&self) -> &[Attachment]
	{
		&self.inner.attachments
	}

	/*

	/// Attach additional context to the error.
	#[track_caller]
	pub fn attach(mut self, attachment: impl fmt::Display) -> Self
	{
		self.inner.attachments.push(Attachment {
			location: *Location::caller(),
			message: attachment.to_string().into_boxed_str(),
		});

		self
	}

	*/

	/// Returns the appropriate HTTP status code to use in an error response.
	fn status(&self) -> StatusCode
	{
		match self.inner.kind {
			ErrorKind::NoContent => StatusCode::NO_CONTENT,
			ErrorKind::BadRequest(_) => StatusCode::BAD_REQUEST,
			ErrorKind::Unauthorized(_) => StatusCode::UNAUTHORIZED,
			ErrorKind::NotFound { .. } => StatusCode::NOT_FOUND,
			ErrorKind::Database(_) | ErrorKind::Panic => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	/// Indicate that an HTTP panicked but the panic was caught.
	#[track_caller]
	pub(crate) fn panic() -> Self
	{
		Self::new(ErrorKind::Panic)
	}

	/// Respond with no data.
	#[track_caller]
	pub(crate) fn no_content() -> Self
	{
		Self::new(ErrorKind::NoContent)
	}

	/// Reject a request because it was malformed in some way.
	#[track_caller]
	pub(crate) fn bad_request(reason: impl Into<BoxError>) -> Self
	{
		Self::new(ErrorKind::BadRequest(reason.into()))
	}

	/// Reject a request because the user is not authenticated / authorized.
	#[track_caller]
	pub(crate) fn unauthorized(reason: impl Into<BoxError>) -> Self
	{
		Self::new(ErrorKind::Unauthorized(reason.into()))
	}

	/// Reject a request because a requested resource was not found.
	#[track_caller]
	pub(crate) fn not_found(what: impl fmt::Display) -> Self
	{
		Self::new(ErrorKind::NotFound { what: what.to_string() })
	}
}

impl fmt::Debug for Error
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		let mut attachments = self.attachments().iter().rev();

		let Some(latest) = attachments.next() else {
			return write!(f, "[{}]: {}", self.source_location(), self.source());
		};

		write!(f, "[{}]: {}", latest.location, latest.message)?;

		for Attachment { location, message } in attachments {
			write!(f, "\n  - [{location}]: {message}")?;
		}

		Ok(())
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(self.source(), f)
	}
}

impl IntoResponse for Error
{
	fn into_response(self) -> Response
	{
		#[derive(Debug, Serialize)]
		#[allow(clippy::missing_docs_in_private_items)]
		struct ErrorResponse
		{
			message: String,

			#[cfg(not(feature = "production"))]
			#[serde(skip_serializing_if = "Option::is_none")]
			debug_info: Option<Vec<String>>,
		}

		let status = self.status();

		if status == StatusCode::INTERNAL_SERVER_ERROR {
			tracing::error! {
				target: "cs2kz_api::audit_log",
				kind = ?self.inner.kind,
				source_location = %self.source_location(),
				context = ?self.attachments()
					.iter()
					.map(|attachment| format!("{attachment:?}"))
					.collect_vec(),

				"internal server error: \"{self}\"",
			};
		} else if cfg!(feature = "production") {
			tracing::debug! {
				%status,
				source_location = %self.source_location(),
				"runtime error: \"{self}\"",
			};
		} else {
			tracing::debug! {
				%status,
				source_location = %self.source_location(),
				context = ?self.attachments()
					.iter()
					.map(|attachment| format!("{attachment:?}"))
					.collect_vec(),

				"runtime error: \"{self}\"",
			};
		}

		#[allow(unused_mut)]
		let mut response = ErrorResponse {
			message: self.to_string(),

			#[cfg(not(feature = "production"))]
			debug_info: None,
		};

		#[cfg(not(feature = "production"))]
		for attachment in self.attachments().iter().rev() {
			response
				.debug_info
				.get_or_insert_with(|| Vec::with_capacity(self.attachments().len()))
				.push(format!("{attachment:?}"));
		}

		(status, Json(response)).into_response()
	}
}

impl From<sqlx::Error> for Error
{
	#[track_caller]
	fn from(value: sqlx::Error) -> Self
	{
		Self::new(value.into())
	}
}

/// The actual representation of [`Error`].
struct Inner
{
	/// Which particular error we're dealing with.
	kind: ErrorKind,

	/// The source code location of where this [`Error`] was created.
	source_location: Location<'static>,

	/// List of attachments to provide better context when debugging.
	attachments: Vec<Attachment>,
}

impl Inner
{
	/// Create a new [`Inner`].
	#[track_caller]
	fn new(kind: ErrorKind) -> Self
	{
		Self { kind, source_location: *Location::caller(), attachments: Vec::new() }
	}
}

/// An attachment to an [`Error`] that can provide more information for
/// debugging.
pub struct Attachment
{
	/// The source code location of where this attachment was created.
	location: Location<'static>,

	/// The message that was attached.
	message: Box<str>,
}

impl fmt::Debug for Attachment
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "[{}]: {}", self.location, self.message)
	}
}

impl fmt::Display for Attachment
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "{} (at {})", self.message, self.location)
	}
}

/*

/// Extension trait for [`Result<T, E>`] that allows attaching additional
/// information to the `Err` variant if there is an error.
#[sealed]
pub trait Context<T>
{
	/// Attach additional context to an error if there is one.
	fn attach(self, attachment: impl fmt::Display) -> Result<T>;
}

#[sealed]
impl<T, E> Context<T> for Result<T, E>
where
	E: Into<Error>,
{
	fn attach(self, attachment: impl fmt::Display) -> Result<T>
	{
		match self {
			Ok(value) => Ok(value),
			Err(error) => Err(error.into().attach(attachment)),
		}
	}
}

*/
