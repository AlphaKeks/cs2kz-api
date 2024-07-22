//! The actual [`tower::Service`] implementation for this service.

use std::cmp;

use axum::extract::FromRef;
use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool, QueryBuilder};
use tap::{Tap, TryConv};

use crate::database::{
	FilteredQueryBuilder,
	QueryBuilderExt,
	SqlErrorExt,
	TransactionExt,
	UpdateQueryBuilder,
};
use crate::services::plugin::PluginVersionID;
use crate::util::ServerIdentifier;

mod error;
pub use error::{Error, Result};

mod models;
pub use models::{
	ApiKey,
	DeleteKeyRequest,
	DeleteKeyResponse,
	FetchServerRequest,
	FetchServerResponse,
	FetchServersRequest,
	FetchServersResponse,
	GenerateAccessTokenRequest,
	GenerateAccessTokenResponse,
	RegisterServerRequest,
	RegisterServerResponse,
	ResetKeyRequest,
	ResetKeyResponse,
	ServerID,
	ServerInfo,
	UpdateServerRequest,
	UpdateServerResponse,
};

mod queries;
mod http;

/// A service for managing KZ servers.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct ServerService
{
	database: Pool<MySql>,
}

impl ServerService
{
	/// Create a new [`ServerService`].
	pub fn new(database: Pool<MySql>) -> Self
	{
		Self { database }
	}

	/// Fetch information about a server.
	async fn fetch_server(&mut self, req: FetchServerRequest)
	-> Result<Option<FetchServerResponse>>
	{
		let mut query = QueryBuilder::new(queries::SELECT).tap_mut(|query| {
			query.push(" WHERE ");
		});

		match req.identifier {
			ServerIdentifier::ID(server_id) => {
				query.push("s.id = ").push_bind(server_id);
			}

			ServerIdentifier::Name(name) => {
				query.push("s.name LIKE ").push_bind(format!("%{name}%"));
			}
		}

		query.push_limits(1, 0);

		let server = query
			.build_query_as::<FetchServerResponse>()
			.fetch_optional(&self.database)
			.await?;

		Ok(server)
	}

	/// Fetch information about servers.
	async fn fetch_servers(&mut self, req: FetchServersRequest) -> Result<FetchServersResponse>
	{
		let mut txn = self.database.begin().await?;
		let mut query = FilteredQueryBuilder::new(queries::SELECT);

		if let Some(name) = req.name {
			query.filter(" s.name LIKE ", format!("%{name}%"));
		}

		if let Some(host) = req.host {
			query.filter(" s.host = ", host);
		}

		if let Some(owner) = req.owned_by {
			if let Some(owner_id) = owner.resolve_id(txn.as_mut()).await? {
				query.filter(" o.id = ", owner_id);
			}
		}

		if let Some(created_after) = req.created_after {
			query.filter(" s.created_on > ", created_after);
		}

		if let Some(created_before) = req.created_before {
			query.filter(" s.created_on < ", created_before);
		}

		query.push_limits(*req.limit, *req.offset);

		let servers = query
			.build_query_as::<FetchServerResponse>()
			.fetch_all(txn.as_mut())
			.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchServersResponse { servers, total })
	}

	/// Register a new server.
	async fn register_server(
		&mut self,
		req: RegisterServerRequest,
	) -> Result<RegisterServerResponse>
	{
		let mut txn = self.database.begin().await?;
		let api_key = ApiKey::new();

		let server_id = sqlx::query! {
			r"
			INSERT INTO
			  Servers (name, host, port, owner_id, refresh_key)
			VALUES
			  (?, ?, ?, ?, ?)
			",
			req.name,
			req.host,
			req.port,
			req.owner_id,
			api_key,
		}
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("owner_id") {
				Error::ServerOwnerDoesNotExist { steam_id: req.owner_id }
			} else {
				Error::Database(error)
			}
		})?
		.last_insert_id()
		.try_conv::<ServerID>()
		.expect("in-range ID");

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			%api_key,
			%server_id,
			owner_id = %req.owner_id,
			"registered new server",
		};

		Ok(RegisterServerResponse { server_id, api_key })
	}

	/// Update a server.
	async fn update_server(&mut self, req: UpdateServerRequest) -> Result<UpdateServerResponse>
	{
		if req.is_empty() {
			return Ok(UpdateServerResponse);
		}

		let mut txn = self.database.begin().await?;
		let mut query = UpdateQueryBuilder::new("Servers");

		if let Some(new_name) = req.new_name {
			query.set("name", new_name);
		}

		if let Some(new_host) = req.new_host {
			query.set("host", new_host);
		}

		if let Some(new_port) = req.new_port {
			query.set("port", new_port);
		}

		if let Some(new_owner_id) = req.new_owner {
			query.set("owner_id", new_owner_id);
		}

		query.push(" WHERE id = ").push_bind(req.server_id);

		let query_result = query.build().execute(txn.as_mut()).await?;

		match query_result.rows_affected() {
			0 => return Err(Error::ServerDoesNotExist { server_id: req.server_id }),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			server_id = %req.server_id,
			"updated server",
		};

		Ok(UpdateServerResponse)
	}

	/// Resets a server's API key.
	async fn reset_key(&mut self, req: ResetKeyRequest) -> Result<ResetKeyResponse>
	{
		let mut txn = self.database.begin().await?;
		let new_key = ApiKey::new();

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Servers
			SET
			  refresh_key = ?
			WHERE
			  id = ?
			",
			new_key,
			req.server_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::ServerDoesNotExist { server_id: req.server_id }),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			server_id = %req.server_id,
			%new_key,
			"reset API key for server",
		};

		Ok(ResetKeyResponse { key: new_key })
	}

	/// Delete a server's API key.
	async fn delete_key(&mut self, req: DeleteKeyRequest) -> Result<DeleteKeyResponse>
	{
		let mut txn = self.database.begin().await?;

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Servers
			SET
			  refresh_key = NULL
			WHERE
			  id = ?
			",
			req.server_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::ServerDoesNotExist { server_id: req.server_id }),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			server_id = %req.server_id,
			"deleted API key of server",
		};

		Ok(DeleteKeyResponse)
	}

	/// Generate a temporary access token for a CS2 server.
	async fn generate_access_token(
		&mut self,
		req: GenerateAccessTokenRequest,
	) -> Result<GenerateAccessTokenResponse>
	{
		let mut txn = self.database.begin().await?;

		let server_info = sqlx::query! {
			r"
			SELECT
			  s.id `server_id: ServerID`,
			  v.id `plugin_version_id: PluginVersionID`
			FROM
			  Servers s
			  JOIN PluginVersions v ON v.semver = ?
			  AND s.refresh_key = ?
			",
			req.plugin_version,
			req.key,
		}
		.fetch_optional(txn.as_mut())
		.await?
		.ok_or(Error::InvalidKeyOrPluginVersion)?;

		// requires auth service
		let token = todo!();

		txn.commit().await?;

		tracing::trace!(server_id = %server_info.server_id, %token, "generated jwt");

		Ok(GenerateAccessTokenResponse { token })
	}
}
