use futures::stream::{Stream, TryStreamExt};
use time::OffsetDateTime;

use crate::database::{self, RowStream};
use crate::pagination::{Limit, Offset, PaginationResults};
use crate::servers::{AccessKey, ServerHost, ServerID, ServerName};
use crate::users::UserID;

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	server_id: ServerID,
) -> database::Result<Option<Server>> {
	let server = sqlx::query!(
		"SELECT
		   s.id `id: ServerID`,
		   s.name `name: ServerName`,
		   s.host `host: ServerHost`,
		   s.port,
		   ou.id `owner_id: UserID`,
		   op.name owner_name,
		   s.created_at,
		   s.last_seen_at
		 FROM Servers s
		 JOIN Users ou ON ou.id = s.owner_id
		 LEFT JOIN Players op ON op.id = s.owner_id
		 WHERE s.id = ?
		 ORDER BY s.id DESC",
		server_id,
	)
	.fetch_optional(conn)
	.await?
	.map(|row| macros::map_row!(row));

	Ok(server)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_name(
	conn: &mut database::Connection,
	server_name: &ServerName,
) -> database::Result<Option<Server>> {
	let server = sqlx::query!(
		"SELECT
		   s.id `id: ServerID`,
		   s.name `name: ServerName`,
		   s.host `host: ServerHost`,
		   s.port,
		   ou.id `owner_id: UserID`,
		   op.name owner_name,
		   s.created_at,
		   s.last_seen_at
		 FROM Servers s
		 JOIN Users ou ON ou.id = s.owner_id
		 LEFT JOIN Players op ON op.id = s.owner_id
		 WHERE s.name LIKE ?
		 ORDER BY s.id DESC",
		format!("%{server_name}%"),
	)
	.fetch_optional(conn)
	.await?
	.map(|row| macros::map_row!(row));

	Ok(server)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_access_key(
	conn: &mut database::Connection,
	access_key: AccessKey,
) -> database::Result<Option<Server>> {
	let server = sqlx::query!(
		"SELECT
		   s.id `id: ServerID`,
		   s.name `name: ServerName`,
		   s.host `host: ServerHost`,
		   s.port,
		   ou.id `owner_id: UserID`,
		   op.name owner_name,
		   s.created_at,
		   s.last_seen_at
		 FROM Servers s
		 JOIN Users ou ON ou.id = s.owner_id
		 LEFT JOIN Players op ON op.id = s.owner_id
		 WHERE s.access_key = ?",
		access_key,
	)
	.fetch_optional(conn)
	.await?
	.map(|row| macros::map_row!(row));

	Ok(server)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get<'c>(
	conn: &'c mut database::Connection,
	GetServersParams {
		name,
		host,
		owner_id,
		limit,
		offset,
	}: GetServersParams<'_>,
) -> database::Result<PaginationResults<impl RowStream<'c, Server>>> {
	let total: u64 = sqlx::query_scalar!(
		"SELECT COUNT(id)
		 FROM Servers
		 WHERE name LIKE COALESCE(?, name)
		 AND host = COALESCE(?, host)
		 AND owner_id = COALESCE(?, owner_id)",
		name,
		host,
		owner_id,
	)
	.fetch_one(&mut *conn)
	.await?
	.try_into()
	.expect("`COUNT()` should return a positive value");

	let stream = sqlx::query!(
		"SELECT
		   s.id `id: ServerID`,
		   s.name `name: ServerName`,
		   s.host `host: ServerHost`,
		   s.port,
		   ou.id `owner_id: UserID`,
		   op.name owner_name,
		   s.created_at,
		   s.last_seen_at
		 FROM Servers s
		 JOIN Users ou ON ou.id = s.owner_id
		 LEFT JOIN Players op ON op.id = s.owner_id
		 WHERE s.name LIKE COALESCE(?, s.name)
		 AND s.host = COALESCE(?, s.host)
		 AND s.owner_id = COALESCE(?, s.owner_id)
		 ORDER BY s.id DESC
		 LIMIT ?
		 OFFSET ?",
		name,
		host,
		owner_id,
		limit,
		offset,
	)
	.fetch(conn)
	.map_ok(|row| macros::map_row!(row));

	Ok(PaginationResults { total, stream })
}

pub struct GetServersParams<'a> {
	pub name: Option<&'a ServerName>,
	pub host: Option<&'a ServerHost>,
	pub owner_id: Option<UserID>,
	pub limit: Limit,
	pub offset: Offset,
}

pub struct Server {
	pub id: ServerID,
	pub name: ServerName,
	pub host: ServerHost,
	pub port: u16,
	pub owner: ServerOwner,
	pub created_at: OffsetDateTime,
	pub last_seen_at: OffsetDateTime,
}

pub struct ServerOwner {
	pub id: UserID,
	pub name: Option<String>,
}

mod macros {
	macro_rules! map_row {
		($row:ident) => {
			Server {
				id: $row.id,
				name: $row.name,
				host: $row.host,
				port: $row.port,
				owner: ServerOwner {
					id: $row.owner_id,
					name: $row.owner_name,
				},
				created_at: $row.created_at,
				last_seen_at: $row.last_seen_at,
			}
		};
	}

	pub(super) use map_row;
}
