use crate::events::Event;
use crate::servers::{AccessKey, ServerHost, ServerID, ServerName};
use crate::users::UserID;
use crate::{database, events, servers};

#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
pub async fn approve(
	pool: &database::ConnectionPool,
	server: NewServer,
) -> Result<ApprovedServer, ApproveServerError> {
	let mut txn = pool.begin().await?;
	let created_server = servers::database::create(&mut txn, servers::database::NewServer {
		name: &server.name,
		host: &server.host,
		port: server.port,
		owner_id: server.owner_id,
	})
	.await?;

	txn.commit().await?;
	events::dispatch(Event::ServerApproved {
		id: created_server.id,
		name: server.name,
		host: server.host,
		port: server.port,
		owner_id: server.owner_id,
	});

	Ok(ApprovedServer {
		id: created_server.id,
		access_key: created_server.access_key,
	})
}

#[derive(Debug)]
pub struct NewServer {
	pub name: ServerName,
	pub host: ServerHost,
	pub port: u16,
	pub owner_id: UserID,
}

#[derive(Debug)]
pub struct ApprovedServer {
	pub id: ServerID,
	pub access_key: AccessKey,
}

#[derive(Debug, Error)]
pub enum ApproveServerError {
	#[error("a server with this name already exists")]
	DuplicateName,

	#[error("a server with this host/port combination already exists")]
	DuplicateHostAndPort,

	#[error("there is no user with the specified owner ID")]
	OwnerDoesNotExist,

	#[error("database error: {0}")]
	Database(#[from] database::Error),
}

impl_error_from!(servers::database::CreateServerError => ApproveServerError => {
	E::DuplicateName => Self::DuplicateName,
	E::DuplicateHostAndPort => Self::DuplicateHostAndPort,
	E::OwnerDoesNotExist => Self::OwnerDoesNotExist,
	E::Database(source) => Self::Database(source),
});
