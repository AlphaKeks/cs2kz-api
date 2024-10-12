use crate::database;
use crate::servers::{ServerHost, ServerID, ServerName};
use crate::users::UserID;

/// Updates a server.
///
/// Returns whether a row has actually been updated.
#[instrument(skip(conn), err(level = "debug"))]
pub async fn update(
	conn: &mut database::Connection,
	ServerUpdate {
		server_id,
		name,
		host,
		port,
		owner_id,
	}: ServerUpdate<'_>,
) -> Result<bool, UpdateServerError> {
	sqlx::query!(
		"UPDATE Servers
		 SET name = COALESCE(?, name),
		     host = COALESCE(?, host),
		     port = COALESCE(?, port),
		     owner_id = COALESCE(?, owner_id)
		 WHERE id = ?",
		name,
		host,
		port,
		owner_id,
		server_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 server ({n})"),
	})
	.map_err(|error| {
		use sqlx::error::ErrorKind as E;

		match error
			.as_database_error()
			.map(|error| (error.kind(), error.message()))
		{
			Some((E::UniqueViolation, message)) if message.contains("`name`") => {
				UpdateServerError::DuplicateName
			}
			Some((E::UniqueViolation, message)) if message.contains("`UC_host_port`") => {
				UpdateServerError::DuplicateHostAndPort
			}
			Some((E::ForeignKeyViolation, message)) if message.contains("`owner_id`") => {
				UpdateServerError::OwnerDoesNotExist
			}
			_ => UpdateServerError::Database(error),
		}
	})
}

/// Updates a server's `last_seen_at` column.
///
/// Returns whether a row has actually been updated.
#[instrument(skip(conn), ret(level = "debug"), err(level = "debug"))]
pub async fn mark_seen(
	conn: &mut database::Connection,
	server_id: ServerID,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE Servers
		 SET last_seen_at = NOW()
		 WHERE id = ?",
		server_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 server ({n})"),
	})
}

pub struct ServerUpdate<'a> {
	pub server_id: ServerID,
	pub name: Option<&'a ServerName>,
	pub host: Option<&'a ServerHost>,
	pub port: Option<u16>,
	pub owner_id: Option<UserID>,
}

#[derive(Debug, Error)]
pub enum UpdateServerError {
	#[error("a server with this name already exists")]
	DuplicateName,

	#[error("a server with this host/port combination already exists")]
	DuplicateHostAndPort,

	#[error("there is no user with the specified owner ID")]
	OwnerDoesNotExist,

	#[error(transparent)]
	Database(#[from] database::Error),
}
