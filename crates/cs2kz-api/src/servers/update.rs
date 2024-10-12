use crate::events::Event;
use crate::servers::{AccessKey, ServerHost, ServerID, ServerName};
use crate::users::UserID;
use crate::{database, events, servers};

#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
pub async fn update(
	pool: &database::ConnectionPool,
	update: ServerUpdate,
) -> Result<bool, UpdateServerError> {
	let mut txn = pool.begin().await?;
	let updated = servers::database::update(&mut txn, servers::database::ServerUpdate {
		server_id: update.server_id,
		name: update.name.as_ref(),
		host: update.host.as_ref(),
		port: update.port,
		owner_id: update.owner_id,
	})
	.await?;

	if updated {
		txn.commit().await?;
		events::dispatch(Event::ServerUpdated {
			id: update.server_id,
		});
	}

	Ok(updated)
}

#[derive(Debug)]
pub struct ServerUpdate {
	pub server_id: ServerID,
	pub name: Option<ServerName>,
	pub host: Option<ServerHost>,
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

	#[error("database error: {0}")]
	Database(#[from] database::Error),
}

impl_error_from!(servers::database::UpdateServerError => UpdateServerError => {
	E::DuplicateName => Self::DuplicateName,
	E::DuplicateHostAndPort => Self::DuplicateHostAndPort,
	E::OwnerDoesNotExist => Self::OwnerDoesNotExist,
	E::Database(source) => Self::Database(source),
});
