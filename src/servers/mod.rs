pub use self::{
	host::ServerHost,
	id::{ParseServerIdError, ServerId},
	name::{InvalidServerName, ServerName},
	port::ServerPort,
	session_id::{ParseServerSessionIdError, ServerSessionId},
};
use {
	crate::{
		access_keys::AccessKey,
		database::{self, DatabaseError, DatabaseResult, QueryBuilder},
		game::Game,
		players::{PlayerId, PlayerName},
		plugin::PluginVersionId,
		stream::StreamExt as _,
		time::Timestamp,
		users::{UserId, Username},
	},
	futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt},
	serde::Serialize,
	sqlx::Row,
	std::iter,
	utoipa::ToSchema,
};

mod host;
mod id;
mod name;
mod port;
mod session_id;

#[derive(Debug, Serialize, ToSchema)]
pub struct Server
{
	pub id: ServerId,
	pub name: ServerName,
	pub host: ServerHost,
	pub port: ServerPort,
	pub game: Game,
	pub owner: ServerOwner,
	pub is_global: bool,
	pub connection_info: Option<ConnectionInfo>,
	pub created_at: Timestamp,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectionInfo
{
	pub current_map: Box<str>,
	pub connected_players: Box<[ConnectedPlayerInfo]>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConnectedPlayerInfo
{
	pub id: PlayerId,
	pub name: PlayerName,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServerOwner
{
	pub id: UserId,
	pub name: Username,
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: Option<&str>,
	host: Option<&str>,
	game: Game,
	owned_by: Option<UserId>,
	#[builder(default = true)] require_access_key: bool,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(*)
		 FROM Servers
		 WHERE name LIKE COALESCE(?, name)
		 AND host = COALESCE(?, host)
		 AND game = ?
		 AND owner_id = COALESCE(?, owner_id)
		 AND (? OR access_key IS NOT NULL)",
		name.map(|name| format!("%{name}%")),
		host,
		game,
		owned_by,
		!require_access_key,
	)
	.fetch_one(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.and_then(async |row| row.try_into().map_err(DatabaseError::convert_count))
	.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get(
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: Option<&str>,
	host: Option<&str>,
	owned_by: Option<UserId>,
	#[builder(default = true)] require_access_key: bool,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Server>>
{
	sqlx::query!(
		"SELECT
		   s.id AS `id: ServerId`,
		   s.name AS `name: ServerName`,
		   s.host AS `host: ServerHost`,
		   s.port AS `port: ServerPort`,
		   s.game AS `game: Game`,
		   o.id AS `owner_id: UserId`,
		   o.name AS `owner_name: Username`,
		   (s.access_key IS NOT NULL) AS `is_global: bool`,
		   s.created_at AS `created_at: Timestamp`,
		   MATCH (s.name) AGAINST (?) AS name_score
		 FROM Servers AS s
		 INNER JOIN Users AS o ON o.id = s.owner_id
		 WHERE s.name LIKE COALESCE(?, s.name)
		 AND s.host = COALESCE(?, s.host)
		 AND s.game = ?
		 AND s.owner_id = COALESCE(?, s.owner_id)
		 AND (? OR s.access_key IS NOT NULL)
		 ORDER BY name_score DESC, s.id DESC
		 LIMIT ?, ?",
		name,
		name.map(|name| format!("%{name}%")),
		host,
		game,
		owned_by,
		!require_access_key,
		offset,
		limit,
	)
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
	.map_ok(|row| Server {
		id: row.id,
		name: row.name,
		host: row.host,
		port: row.port,
		game: row.game,
		owner: ServerOwner { id: row.owner_id, name: row.owner_name },
		is_global: row.is_global,
		connection_info: None,
		created_at: row.created_at,
	})
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] server_id: ServerId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<Server>>
{
	sqlx::query!(
		"SELECT
		   s.id AS `id: ServerId`,
		   s.name AS `name: ServerName`,
		   s.host AS `host: ServerHost`,
		   s.port AS `port: ServerPort`,
		   s.game AS `game: Game`,
		   o.id AS `owner_id: UserId`,
		   o.name AS `owner_name: Username`,
		   (s.access_key IS NOT NULL) AS `is_global: bool`,
		   s.created_at AS `created_at: Timestamp`
		 FROM Servers AS s
		 INNER JOIN Users AS o ON o.id = s.owner_id
		 WHERE s.id = ?",
		server_id,
	)
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.map_ok(|maybe_row| {
		maybe_row.map(|row| Server {
			id: row.id,
			name: row.name,
			host: row.host,
			port: row.port,
			game: row.game,
			owner: ServerOwner { id: row.owner_id, name: row.owner_name },
			is_global: row.is_global,
			connection_info: None,
			created_at: row.created_at,
		})
	})
	.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_id_by_access_key(
	#[builder(start_fn)] access_key: AccessKey,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<ServerId>>
{
	sqlx::query_scalar!("SELECT id AS `id: ServerId` FROM Servers WHERE access_key = ?", access_key)
		.fetch_optional(db_conn.raw_mut())
		.map_err(DatabaseError::from)
		.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_owner_id(
	#[builder(start_fn)] server_id: ServerId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<UserId>>
{
	sqlx::query_scalar!(
		"SELECT owner_id AS `owner_id: UserId`
		 FROM Servers
		 WHERE id = ?",
		server_id,
	)
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.await
}

#[derive(Debug)]
pub struct CreatedServer
{
	pub id: ServerId,
	pub access_key: AccessKey,
}

#[derive(Debug, Display, Error, From)]
pub enum CreateServerError
{
	NameAlreadyInUse,
	HostAndPortAlreadyInUse,
	DatabaseError(DatabaseError),
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: ServerName,
	host: ServerHost,
	port: ServerPort,
	game: Game,
	owned_by: UserId,
	#[builder(default = AccessKey::new())] access_key: AccessKey,
) -> Result<CreatedServer, CreateServerError>
{
	sqlx::query!(
		"INSERT INTO Servers (name, host, port, game, owner_id, access_key)
		 VALUES (?, ?, ?, ?, ?, ?)
		 RETURNING id",
		name,
		host,
		port,
		game,
		owned_by,
		access_key,
	)
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get(0))
	.map_ok(|id| CreatedServer { id, access_key })
	.map_err(DatabaseError::from)
	.map_err(|err| {
		if err.is_unique_violation("name") {
			CreateServerError::NameAlreadyInUse
		} else if err.is_unique_violation("host") {
			CreateServerError::HostAndPortAlreadyInUse
		} else {
			CreateServerError::DatabaseError(err)
		}
	})
	.await
}

#[derive(Debug, Display, Error, From)]
pub enum UpdateServerError
{
	NameAlreadyInUse,
	HostAndPortAlreadyInUse,
	DatabaseError(DatabaseError),
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn update(
	#[builder(start_fn)] server_id: ServerId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: Option<ServerName>,
	host: Option<ServerHost>,
	port: Option<ServerPort>,
	game: Option<Game>,
) -> Result<bool, UpdateServerError>
{
	sqlx::query!(
		"UPDATE Servers
		 SET name = COALESCE(?, name),
		     host = COALESCE(?, host),
		     port = COALESCE(?, port),
		     game = COALESCE(?, game)
		 WHERE id = ?",
		name,
		host,
		port,
		game,
		server_id,
	)
	.execute(db_conn.raw_mut())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.map_err(|err| {
		if err.is_unique_violation("name") {
			UpdateServerError::NameAlreadyInUse
		} else if err.is_unique_violation("host") {
			UpdateServerError::HostAndPortAlreadyInUse
		} else {
			UpdateServerError::DatabaseError(err)
		}
	})
	.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	owned_by: Option<UserId>,
) -> DatabaseResult<u64>
{
	sqlx::query!(
		"DELETE FROM Servers
		 WHERE owner_id = COALESCE(?, owner_id)
		 LIMIT ?",
		owned_by,
		count,
	)
	.execute(db_conn.raw_mut())
	.map_ok(|query_result| query_result.rows_affected())
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn reset_access_key(
	#[builder(start_fn)] server_id: ServerId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	#[builder(default = AccessKey::new())] access_key: AccessKey,
) -> DatabaseResult<Option<AccessKey>>
{
	sqlx::query!(
		"UPDATE Servers
		 SET access_key = ?
		 WHERE id = ?",
		access_key,
		server_id,
	)
	.execute(db_conn.raw_mut())
	.map_ok(|query_result| (query_result.rows_affected() > 0).then_some(access_key))
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn, servers), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete_access_key(
	#[builder(start_fn)] servers: impl IntoIterator<Item = ServerId>,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<u64>
{
	let mut iter = servers.into_iter();

	let Some(first) = iter.next() else {
		return Ok(0);
	};

	let mut query = QueryBuilder::new("UPDATE Servers SET access_key = NULL WHERE id IN");

	query.push_tuples(iter::once(first).chain(iter), |mut query, server_id| {
		query.push_bind(server_id);
	});

	query
		.build()
		.persistent(false)
		.execute(db_conn.raw_mut())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}

/// Updates the `last_seen_at` value for all given servers.
///
/// Returns the number of servers updated.
#[instrument(skip(db_conn, servers), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn mark_seen(
	#[builder(start_fn)] servers: &[ServerId],
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<u64>
{
	if servers.is_empty() {
		return Ok(0);
	}

	let mut query = QueryBuilder::new("UPDATE Servers SET last_seen_at = NOW() WHERE id IN");

	query.push_tuples(servers, |mut query, &server_id| {
		query.push_bind(server_id);
	});

	query
		.build()
		.persistent(false)
		.execute(db_conn.raw_mut())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create_session(
	#[builder(start_fn)] server_id: ServerId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	plugin_version_id: PluginVersionId,
) -> DatabaseResult<ServerSessionId>
{
	let servers_updated = mark_seen(&[server_id]).exec(db_conn).await?;
	debug_assert_eq!(servers_updated, 1);

	sqlx::query!(
		"INSERT INTO ServerSessions (server_id, plugin_version_id)
		 VALUES (?, ?)
		 RETURNING id",
		server_id,
		plugin_version_id,
	)
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get(0))
	.map_err(DatabaseError::from)
	.await
}
