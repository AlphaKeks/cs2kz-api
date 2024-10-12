use sqlx::Row;

use crate::database::{self, ErrorExt};
use crate::servers::{AccessKey, ServerHost, ServerID, ServerName};
use crate::users::UserID;

#[instrument(skip(conn), ret(level = "debug"), err(level = "debug"))]
pub async fn create(
	conn: &mut database::Connection,
	NewServer {
		name,
		host,
		port,
		owner_id,
	}: NewServer<'_>,
) -> Result<CreatedServer, CreateServerError> {
	let access_key = AccessKey::new();
	let id = sqlx::query!(
		"INSERT INTO Servers (name, host, port, owner_id, access_key)
		 VALUES (?, ?, ?, ?, ?)
		 RETURNING id",
		name,
		host,
		port,
		owner_id,
		access_key,
	)
	.fetch_one(conn)
	.await
	.and_then(|row| row.try_get(0))
	.map_err(|error| {
		use sqlx::error::ErrorKind as E;

		match error
			.as_database_error()
			.map(|error| (error.kind(), error.message()))
		{
			Some((E::UniqueViolation, message)) if message.contains("`name`") => {
				CreateServerError::DuplicateName
			}
			Some((E::UniqueViolation, message)) if message.contains("`UC_host_port`") => {
				CreateServerError::DuplicateHostAndPort
			}
			Some((E::ForeignKeyViolation, message)) if message.contains("`owner_id`") => {
				CreateServerError::OwnerDoesNotExist
			}
			_ => CreateServerError::Database(error),
		}
	})?;

	Ok(CreatedServer { id, access_key })
}

pub struct NewServer<'a> {
	pub name: &'a ServerName,
	pub host: &'a ServerHost,
	pub port: u16,
	pub owner_id: UserID,
}

#[derive(Debug)]
pub struct CreatedServer {
	pub id: ServerID,
	pub access_key: AccessKey,
}

#[derive(Debug, Error)]
pub enum CreateServerError {
	#[error("a server with this name already exists")]
	DuplicateName,

	#[error("a server with this host/port combination already exists")]
	DuplicateHostAndPort,

	#[error("there is no user with the specified owner ID")]
	OwnerDoesNotExist,

	#[error(transparent)]
	Database(#[from] database::Error),
}
