//! Everything related to authorization.

use std::future::Future;

use axum::extract::rejection::PathRejection;
use axum::http::request;
use axum::response::{IntoResponse, Response};
use sqlx::{MySql, Transaction};
use thiserror::Error;

use crate::authentication;
use crate::http::HandlerError;

mod permissions;

#[doc(inline)]
pub use permissions::Permissions;

mod none;

#[doc(inline)]
pub use none::None;

mod has_permissions;

#[doc(inline)]
pub use has_permissions::HasPermissions;

mod is_server_admin_or_owner;

#[doc(inline)]
pub use is_server_admin_or_owner::IsServerAdminOrOwner;

/// Used for deciding an authorization strategy when doing [session authentication].
///
/// [session authentication]: crate::authentication::session
pub trait AuthorizeSession: Send + Sync + 'static {
	/// Authorize a session for the given `user`.
	fn authorize_session(
		user: &authentication::User,
		req: &mut request::Parts,
		transaction: &mut Transaction<'static, MySql>,
	) -> impl Future<Output = Result<(), AuthorizeError>> + Send;
}

/// The different types of errors that can occur while authorizing a session.
#[derive(Debug, Error)]
pub enum AuthorizeError {
	/// The user does not have a session ID cookie.
	#[error("missing session ID")]
	MissingSessionID,

	/// The user does have a session ID cookie, but it is not a valid UUID.
	#[error("invalid session ID: {0}")]
	InvalidSessionID(uuid::Error),

	/// The user does have a session ID cookie, but it is not in the database or has expired.
	#[error("session does not exist or has expired")]
	InvalidSession,

	/// The user does have a valid session ID cookie, but lacks the required permissions.
	#[error("you have insufficient permissions")]
	InsufficientPermissions,

	/// The user requested to update a server they don't own, and they're also not an admin
	#[error("you are not the owner of this server")]
	NotServerOwner,

	/// Extraction of a path parameter failed.
	#[error(transparent)]
	MissingPathParameter(#[from] PathRejection),

	/// A database error.
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}

impl From<AuthorizeError> for HandlerError {
	fn from(error: AuthorizeError) -> Self {
		let response = Self::unauthorized().with_message(error.to_string());

		match error {
			AuthorizeError::InvalidSessionID(source) => response.with_source(source),
			AuthorizeError::MissingPathParameter(rejection) => response.with_source(rejection),
			AuthorizeError::Database(source) => Self::internal_server_error().with_source(source),
			AuthorizeError::MissingSessionID
			| AuthorizeError::InvalidSession
			| AuthorizeError::InsufficientPermissions
			| AuthorizeError::NotServerOwner => response,
		}
	}
}

impl IntoResponse for AuthorizeError {
	fn into_response(self) -> Response {
		HandlerError::from(self).into_response()
	}
}
