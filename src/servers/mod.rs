//! Everything related to KZ servers.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::sync::Arc;
use std::time::Duration;

use axum::extract::FromRef;
use sqlx::{MySql, Pool, QueryBuilder};
use uuid::Uuid;

use crate::authentication::{Jwt, JwtState};
use crate::kz::ServerIdentifier;
use crate::make_id::IntoID;
use crate::plugin::PluginVersionID;
use crate::sqlx::query::{FilteredQuery, QueryBuilderExt, UpdateQuery};
use crate::sqlx::{query, FetchID, SqlErrorExt};
use crate::{authentication, Error, Result};

#[cfg(test)]
mod tests;

mod models;
pub use models::{
	AccessKeyRequest,
	AccessKeyResponse,
	CreatedServer,
	FetchServersRequest,
	Host,
	NewServer,
	RefreshKey,
	Server,
	ServerID,
	ServerInfo,
	ServerUpdate,
};

mod queries;
pub mod http;

/// A service for dealing with KZ servers as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct ServerService
{
	database: Pool<MySql>,
	jwt_state: Arc<JwtState>,
}

impl ServerService
{
	/// Creates a new [`ServerService`] instance.
	pub const fn new(database: Pool<MySql>, jwt_state: Arc<JwtState>) -> Self
	{
		Self { database, jwt_state }
	}

	/// Fetches a single server.
	pub async fn fetch_server(&self, server: ServerIdentifier) -> Result<Server>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push(" WHERE ");

		match server {
			ServerIdentifier::ID(id) => {
				query.push(" s.id = ").push_bind(id);
			}
			ServerIdentifier::Name(name) => {
				query.push(" s.name LIKE ").push_bind(format!("%{name}%"));
			}
		}

		let server = query
			.build_query_as::<Server>()
			.fetch_optional(&self.database)
			.await?
			.ok_or_else(|| Error::not_found("server"))?;

		Ok(server)
	}

	/// Fetches many servers.
	///
	/// The `limit` and `offset` fields in [`FetchServersRequest`] can be used
	/// for pagination. The `u64` part of the returned tuple indicates how many
	/// servers _could_ be fetched; also useful for pagination.
	pub async fn fetch_servers(&self, request: FetchServersRequest) -> Result<(Vec<Server>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = FilteredQuery::new(queries::SELECT);

		if let Some(name) = request.name {
			query.filter(" s.name LIKE ", format!("%{name}%"));
		}

		if let Some(host) = request.host {
			query.filter(" s.host = ", host.to_string());
		}

		if let Some(player) = request.owned_by {
			let steam_id = player.fetch_id(transaction.as_mut()).await?;

			query.filter(" s.owner_id = ", steam_id);
		}

		if let Some(created_after) = request.created_after {
			query.filter(" s.created_on > ", created_after);
		}

		if let Some(created_before) = request.created_before {
			query.filter(" s.created_on < ", created_before);
		}

		query.push_limits(request.limit, request.offset);

		let servers = query
			.build_query_as::<Server>()
			.fetch_all(transaction.as_mut())
			.await?;

		if servers.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((servers, total))
	}

	/// Registers a new global server.
	pub async fn register_server(&self, server: NewServer) -> Result<CreatedServer>
	{
		let mut transaction = self.database.begin().await?;
		let refresh_key = Uuid::new_v4();
		let server_id = sqlx::query! {
			r#"
			INSERT INTO
			  Servers (name, host, port, owner_id, refresh_key)
			VALUES
			  (?, ?, ?, ?, ?)
			"#,
			server.name,
			server.host,
			server.port,
			server.owned_by,
			refresh_key,
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("owner_id") {
				Error::not_found("server owner").context(err)
			} else {
				Error::from(err)
			}
		})?
		.last_insert_id()
		.into_id::<ServerID>()?;

		transaction.commit().await?;

		tracing::debug! {
			target: "cs2kz_api::audit_log",
			id = %server_id,
			%refresh_key,
			"created new server",
		};

		Ok(CreatedServer { server_id, refresh_key })
	}

	/// Update an existing server.
	pub async fn update_server(&self, server_id: ServerID, update: ServerUpdate) -> Result<()>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = UpdateQuery::new("Servers");

		if let Some(name) = update.name {
			query.set("name", name);
		}

		if let Some(host) = update.host {
			query.set("host", host);
		}

		if let Some(port) = update.port {
			query.set("port", port);
		}

		if let Some(steam_id) = update.owned_by {
			query.set("owner_id", steam_id);
		}

		query.push(" WHERE id = ").push_bind(server_id);

		let query_result = query.build().execute(transaction.as_mut()).await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("server")),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		transaction.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			%server_id,
			"updated server",
		};

		Ok(())
	}

	/// Generates a temporary access token for a server.
	pub async fn generate_access_token(
		&self,
		request: AccessKeyRequest,
	) -> Result<AccessKeyResponse>
	{
		let mut transaction = self.database.begin().await?;

		let server = sqlx::query! {
			r#"
			SELECT
			  s.id `server_id: ServerID`,
			  v.id `plugin_version_id: PluginVersionID`
			FROM
			  Servers s
			  JOIN PluginVersions v ON v.semver = ?
			  AND s.refresh_key = ?
			"#,
			request.plugin_version.to_string(),
			request.refresh_key,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| authentication::Server::new(row.server_id, row.plugin_version_id))
		.ok_or_else(|| Error::unauthorized())?;

		let jwt = Jwt::new(&server, Duration::from_secs(60 * 15));
		let access_key = self.jwt_state.encode(jwt)?;

		transaction.commit().await?;

		tracing::debug! {
			server_id = %server.id(),
			%access_key,
			"generated access key for server",
		};

		Ok(AccessKeyResponse { access_key })
	}

	/// Replaces a server's API key with a new randomly generated one.
	///
	/// The new key will be returned by this function and cannot be accessed
	/// again later.
	pub async fn replace_api_key(&self, server_id: ServerID) -> Result<RefreshKey>
	{
		let mut transaction = self.database.begin().await?;
		let refresh_key = Uuid::new_v4();
		let query_result = sqlx::query! {
			r#"
			UPDATE
			  Servers
			SET
			  refresh_key = ?
			WHERE
			  id = ?
			"#,
			refresh_key,
			server_id
		}
		.execute(transaction.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("server")),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		transaction.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			%server_id,
			%refresh_key,
			"generated new API key for server",
		};

		Ok(RefreshKey { refresh_key })
	}

	/// Deletes a server's API key.
	pub async fn delete_api_key(&self, server_id: ServerID) -> Result<()>
	{
		let mut transaction = self.database.begin().await?;

		let query_result = sqlx::query! {
			r#"
			UPDATE
			  Servers
			SET
			  refresh_key = NULL
			WHERE
			  id = ?
			"#,
			server_id,
		}
		.execute(transaction.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("server")),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		transaction.commit().await?;

		tracing::info!(target: "cs2kz_api::audit_log", %server_id, "deleted API key for server");

		Ok(())
	}
}
