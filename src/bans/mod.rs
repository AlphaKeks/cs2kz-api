//! Everything related to KZ player bans.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::net::{IpAddr, Ipv6Addr};
use std::sync::Arc;

use axum::extract::FromRef;
use query::UpdateQuery;
use sqlx::{MySql, MySqlExecutor, Pool, QueryBuilder};
use time::OffsetDateTime;

use crate::authentication::JwtState;
use crate::plugin::PluginVersionID;
use crate::sqlx::query::QueryBuilderExt;
use crate::sqlx::{query, FetchID, FilteredQuery, SqlErrorExt};
use crate::{authentication, Error, Result};

mod models;
pub use models::{
	Ban,
	BanID,
	BanReason,
	BanUpdate,
	CreatedBan,
	CreatedUnban,
	FetchBansRequest,
	NewBan,
	NewUnban,
	Unban,
	UnbanID,
};

mod queries;
pub mod http;

/// A service for dealing with KZ records as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct BanService
{
	database: Pool<MySql>,
	jwt_state: Arc<JwtState>,
}

impl BanService
{
	/// Creates a new [`BanService`] instance.
	pub const fn new(database: Pool<MySql>, jwt_state: Arc<JwtState>) -> Self
	{
		Self { database, jwt_state }
	}

	/// Fetches a single ban.
	pub async fn fetch_ban(&self, ban_id: BanID) -> Result<Ban>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push(" WHERE b.id = ").push_bind(ban_id);

		let ban = query
			.build_query_as::<Ban>()
			.fetch_optional(&self.database)
			.await?
			.ok_or_else(|| Error::not_found("ban"))?;

		Ok(ban)
	}

	/// Fetches many bans.
	///
	/// The `limit` and `offset` fields in [`FetchBansRequest`] can be used for
	/// pagination. The `u64` part of the returned tuple indicates how many
	/// bans _could_ be fetched; also useful for pagination.
	pub async fn fetch_bans(&self, request: FetchBansRequest) -> Result<(Vec<Ban>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = FilteredQuery::new(queries::SELECT);

		if let Some(player) = request.player {
			let steam_id = player.fetch_id(transaction.as_mut()).await?;

			query.filter(" b.player_id = ", steam_id);
		}

		if let Some(server) = request.server {
			let server_id = server.fetch_id(transaction.as_mut()).await?;

			query.filter(" b.server_id = ", server_id);
		}

		if let Some(reason) = request.reason {
			query.filter(" b.reason = ", reason);
		}

		if let Some(unbanned) = request.unbanned {
			query.filter_is_null(" ub.id ", !unbanned);
		}

		if let Some(created_after) = request.created_after {
			query.filter(" b.created_on > ", created_after);
		}

		if let Some(created_before) = request.created_before {
			query.filter(" b.created_on < ", created_before);
		}

		query.push_limits(request.limit, request.offset);

		let bans = query
			.build_query_as::<Ban>()
			.fetch_all(transaction.as_mut())
			.await?;

		if bans.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((bans, total))
	}

	/// Submits a new ban.
	pub async fn submit_ban(
		&self,
		ban: NewBan,
		server: Option<authentication::Server>,
		admin: Option<authentication::User>,
	) -> Result<CreatedBan>
	{
		let mut transaction = self.database.begin().await?;

		let (already_banned, previous_offenses) = sqlx::query! {
			r#"
			SELECT
			  COUNT(b1.id) > 0 `already_banned: bool`,
			  COUNT(b2.id) `previous_bans: u8`
			FROM
			  Players p
			  LEFT JOIN Bans b1 ON b1.player_id = p.id
			  AND b1.expires_on > NOW()
			  LEFT JOIN Bans b2 ON b2.player_id = p.id
			  AND b2.expires_on < NOW()
			WHERE
			  p.id = ?
			"#,
			ban.player_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| (row.already_banned, row.previous_bans))
		.ok_or_else(|| Error::not_found("player"))?;

		if already_banned {
			return Err(Error::already_exists("ban"));
		}

		let player_ip = match ban.player_ip {
			Some(IpAddr::V4(ip)) => ip.to_ipv6_mapped(),
			Some(IpAddr::V6(ip)) => ip,
			None => sqlx::query_scalar! {
				r#"
				SELECT
				  ip_address `ip: Ipv6Addr`
				FROM
				  Players
				WHERE
				  id = ?
				"#,
				ban.player_id,
			}
			.fetch_optional(transaction.as_mut())
			.await?
			.ok_or_else(|| Error::not_found("player"))?,
		};

		let plugin_version_id = if let Some(id) = server.map(|server| server.plugin_version_id()) {
			id
		} else {
			sqlx::query_scalar! {
				r#"
				SELECT
				  id `id: PluginVersionID`
				FROM
				  PluginVersions
				ORDER BY
				  created_on DESC
				LIMIT
				  1
				"#,
			}
			.fetch_one(transaction.as_mut())
			.await?
		};

		let expires_on = OffsetDateTime::now_utc() + ban.reason.duration(previous_offenses);

		let ban_id = sqlx::query! {
			r#"
			INSERT INTO
			  Bans (
			    player_id,
			    player_ip,
			    server_id,
			    reason,
			    admin_id,
			    plugin_version_id,
			    expires_on
			  )
			VALUES
			  (?, ?, ?, ?, ?, ?, ?)
			"#,
			ban.player_id,
			player_ip,
			server.map(|server| server.id()),
			ban.reason,
			admin.map(|admin| admin.steam_id()),
			plugin_version_id,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::not_found("player").context(err)
			} else if err.is_fk_violation_of("admin_id") {
				Error::not_found("admin").context(err)
			} else {
				Error::from(err)
			}
		})?
		.last_insert_id()
		.into();

		transaction.commit().await?;

		tracing::trace! {
			target: "cs2kz_api::audit_log",
			%ban_id,
			%ban.player_id,
			?ban.reason,
			?server,
			?admin,
			"created ban",
		};

		Ok(CreatedBan { ban_id })
	}

	/// Updates an existing ban.
	pub async fn update_ban(&self, ban_id: BanID, update: BanUpdate) -> Result<()>
	{
		let mut transaction = self.database.begin().await?;

		if let Some(unban_id) = is_already_unbanned(ban_id, transaction.as_mut()).await? {
			return Err(Error::ban_already_reverted(ban_id, unban_id));
		}

		let mut query = UpdateQuery::new("Bans");

		if let Some(reason) = update.reason {
			query.set(" reason ", reason);
		}

		if let Some(expires_on) = update.expires_on {
			query.set(" expires_on ", expires_on);
		}

		query.push(" WHERE id = ").push_bind(ban_id);

		let query_result = query.build().execute(transaction.as_mut()).await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("ban")),
			n => assert_eq!(n, 1, "updated more than 1 ban"),
		}

		transaction.commit().await?;

		tracing::info!(target: "cs2kz_api::audit_log", %ban_id, "updated ban");

		Ok(())
	}

	/// Submits a new unban / reverts an existing ban.
	pub async fn submit_unban(
		&self,
		ban_id: BanID,
		unban: NewUnban,
		admin: authentication::User,
	) -> Result<CreatedUnban>
	{
		let mut transaction = self.database.begin().await?;

		if let Some(unban_id) = is_already_unbanned(ban_id, transaction.as_mut()).await? {
			return Err(Error::ban_already_reverted(ban_id, unban_id));
		}

		let query_result = sqlx::query! {
			r#"
			UPDATE
			  Bans
			SET
			  expires_on = NOW()
			WHERE
			  id = ?
			"#,
			ban_id,
		}
		.execute(transaction.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("ban")),
			n => assert_eq!(n, 1, "updated more than 1  ban"),
		}

		tracing::info!(target: "cs2kz_api::audit_log", %ban_id, "reverted ban");

		let unban_id = sqlx::query! {
			r#"
			INSERT INTO
			  Unbans (ban_id, reason, admin_id)
			VALUES
			  (?, ?, ?)
			"#,
			ban_id,
			unban.reason,
			admin.steam_id(),
		}
		.execute(transaction.as_mut())
		.await?
		.last_insert_id()
		.into();

		transaction.commit().await?;

		tracing::info!(target: "cs2kz_api::audit_log", %ban_id, %unban_id, "created unban");

		Ok(CreatedUnban { unban_id })
	}
}

/// Checks if a ban has already been reverted, and returns the corresponding
/// [`UnbanID`].
async fn is_already_unbanned(
	ban_id: BanID,
	executor: impl MySqlExecutor<'_>,
) -> Result<Option<UnbanID>>
{
	sqlx::query_scalar! {
		r#"
		SELECT
		  id `id: UnbanID`
		FROM
		  Unbans
		WHERE
		  ban_id = ?
		"#,
		ban_id,
	}
	.fetch_optional(executor)
	.await
	.map_err(Error::from)
}
