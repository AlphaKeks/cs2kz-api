//! A service for managing player bans.

/* TODO:
 * - allow attaching notes to bans
 *    - allow including notes when submitting bans
 *    - allow updating notes when updating bans
 */

use std::net::{IpAddr, Ipv6Addr};
use std::time::Duration;

use axum::extract::FromRef;
use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use sqlx::{MySql, Pool, QueryBuilder, Transaction};
use tap::{Conv, Tap};

use crate::database::{
	FilteredQueryBuilder,
	QueryBuilderExt,
	SqlErrorExt,
	TransactionExt,
	UpdateQueryBuilder,
};
use crate::services::plugin::PluginVersionID;
use crate::services::servers::ServerID;
use crate::services::AuthService;
use crate::util::AddrExt;

mod error;
pub use error::{Error, Result};

mod models;
pub use models::{
	BanID,
	BanReason,
	BanRequest,
	BanResponse,
	BannedBy,
	FetchBanRequest,
	FetchBanResponse,
	FetchBansRequest,
	FetchBansResponse,
	Unban,
	UnbanID,
	UnbanRequest,
	UnbanResponse,
	UpdateBanRequest,
};

mod queries;
mod http;

/// A service for managing player bans.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct BanService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl BanService
{
	/// Create a new [`BanService`].
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetch a ban.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_ban(&self, req: FetchBanRequest) -> Result<Option<FetchBanResponse>>
	{
		let mut query = QueryBuilder::new(queries::SELECT).tap_mut(|query| {
			query.push(" WHERE b.id = ").push_bind(req.ban_id);
			query.push_limits(1, 0);
		});

		let ban = query
			.build_query_as::<FetchBanResponse>()
			.fetch_optional(&self.database)
			.await?;

		Ok(ban)
	}

	/// Fetch many bans.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_bans(&self, req: FetchBansRequest) -> Result<FetchBansResponse>
	{
		let mut txn = self.database.begin().await?;
		let mut query = FilteredQueryBuilder::new(queries::SELECT);

		if let Some(player) = req.player {
			let steam_id = player.resolve_id(txn.as_mut()).await?;

			query.filter(" b.player_id = ", steam_id);
		}

		if let Some(server) = req.server {
			let server_id = server.resolve_id(txn.as_mut()).await?;

			query.filter(" b.server_id = ", server_id);
		}

		if let Some(reason) = req.reason {
			query.filter(" b.reason = ", reason);
		}

		if let Some(unbanned) = req.unbanned {
			query.filter_is_null(" ub.id ", !unbanned);
		}

		if let Some(created_after) = req.created_after {
			query.filter(" b.created_on > ", created_after);
		}

		if let Some(created_before) = req.created_before {
			query.filter(" b.created_on < ", created_before);
		}

		query.push_limits(*req.limit, *req.offset);

		let bans = query
			.build_query_as::<FetchBanResponse>()
			.fetch_all(txn.as_mut())
			.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchBansResponse { bans, total })
	}

	/// Ban a player.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn ban_player(&self, req: BanRequest) -> Result<BanResponse>
	{
		let mut txn = self.database.begin().await?;

		let ban_duration = calculate_ban_duration(req.player_id, req.reason, &mut txn).await?;
		let player_ip = resolve_player_ip(req.player_ip, req.player_id, &mut txn).await?;
		let banned_by_details = banned_by_details(req.banned_by, &mut txn).await?;

		let ban_id = create_ban(
			req.player_id,
			player_ip,
			req.reason,
			&banned_by_details,
			ban_duration,
			&mut txn,
		)
		.await?;

		txn.commit().await?;

		tracing::trace! {
			target: "cs2kz_api::audit_log",
			%ban_id,
			player_id = %req.player_id,
			reason = %req.reason,
			server_id = ?banned_by_details.server_id,
			admin_id = ?banned_by_details.admin_id,
			?ban_duration,
			"issued ban",
		};

		Ok(BanResponse { ban_id })
	}

	/// Update a ban.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn update_ban(&self, req: UpdateBanRequest) -> Result<()>
	{
		if req.is_empty() {
			return Ok(());
		}

		let (created_on, unban_id) = sqlx::query! {
			r"
			SELECT
			  b.created_on `created_on: DateTime<Utc>`,
			  ub.id `unban_id: UnbanID`
			FROM
			  Bans b
			  LEFT JOIN Unbans ub ON ub.ban_id = b.id
			WHERE
			  b.id = ?
			",
			req.ban_id,
		}
		.fetch_optional(&self.database)
		.await?
		.map(|row| (row.created_on, row.unban_id))
		.ok_or(Error::BanDoesNotExist { ban_id: req.ban_id })?;

		if matches!(req.new_expiration_date, Some(date) if date < created_on) {
			return Err(Error::ExpirationBeforeCreation);
		}

		if let Some(unban_id) = unban_id {
			return Err(Error::BanAlreadyReverted { unban_id });
		}

		let mut txn = self.database.begin().await?;
		let mut query = UpdateQueryBuilder::new("Bans");

		if let Some(reason) = req.new_reason {
			query.set("reason", reason);
		}

		if let Some(expiration_date) = req.new_expiration_date {
			query.set("expires_on", expiration_date);
		}

		query.push(" WHERE id = ").push_bind(req.ban_id);

		let query_result = query.build().execute(txn.as_mut()).await?;

		match query_result.rows_affected() {
			0 => return Err(Error::BanDoesNotExist { ban_id: req.ban_id }),
			n => assert_eq!(n, 1, "updated more than 1 ban"),
		}

		txn.commit().await?;

		tracing::debug!(target: "cs2kz_api::audit_log", ban_id = %req.ban_id, "updated ban");

		Ok(())
	}

	/// Unban a player.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn unban_player(&self, req: UnbanRequest) -> Result<UnbanResponse>
	{
		let existing_unban = sqlx::query_scalar! {
			r"
			SELECT
			  id `id: UnbanID`
			FROM
			  Unbans
			WHERE
			  ban_id = ?
			",
			req.ban_id,
		}
		.fetch_optional(&self.database)
		.await?;

		if let Some(unban_id) = existing_unban {
			return Err(Error::BanAlreadyReverted { unban_id });
		}

		let mut txn = self.database.begin().await?;

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Bans
			SET
			  expires_on = NOW()
			WHERE
			  id = ?
			",
			req.ban_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::BanDoesNotExist { ban_id: req.ban_id }),
			n => assert_eq!(n, 1, "updated more than 1 ban"),
		}

		let unban_id = sqlx::query! {
			r"
			INSERT INTO
			  Unbans (ban_id, reason, admin_id)
			VALUES
			  (?, ?, ?)
			",
			req.ban_id,
			req.reason,
			req.admin_id,
		}
		.execute(txn.as_mut())
		.await?
		.last_insert_id()
		.conv::<UnbanID>();

		txn.commit().await?;

		tracing::debug! {
			target: "cs2kz_api::audit_log",
			ban_id = %req.ban_id,
			%unban_id,
			admin_id = %req.admin_id,
			"reverted ban",
		};

		Ok(UnbanResponse { unban_id })
	}
}

/// Calculates the ban duration for a new ban for a given player for a given
/// reason.
async fn calculate_ban_duration(
	player_id: SteamID,
	reason: BanReason,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Duration>
{
	let (currently_banned, has_previous_bans, previous_ban_duration) = sqlx::query! {
		r"
		SELECT
		  COUNT(active_bans.id) > 0 `currently_banned: bool`,
		  COUNT(expired_bans.id) > 0 `has_previous_bans: bool`,
		  TIMESTAMPDIFF(
		    SECOND,
		    expired_bans.created_on,
		    expired_bans.expires_on
		  ) `previous_ban_duration: u64`
		FROM
		  Players p
		  LEFT JOIN Bans active_bans ON active_bans.player_id = p.id
		  AND active_bans.expires_on > NOW()
		  LEFT JOIN Bans expired_bans ON expired_bans.player_id = p.id
		  AND expired_bans.expires_on < NOW()
		  AND expired_bans.id IN (
		    SELECT
		      ban_id
		    FROM
		      Unbans
		    WHERE
		      reason != 'false_ban'
		  )
		WHERE
		  p.id = ?
		",
		player_id,
	}
	.fetch_optional(txn.as_mut())
	.await?
	.map(|row| (row.currently_banned, row.has_previous_bans, row.previous_ban_duration))
	.unwrap_or_default();

	match (currently_banned, has_previous_bans, previous_ban_duration) {
		// This is the player's first ever ban
		(false, false, previous_ban_duration @ None)
		// The player isn't currently banned but was banned in the past
		| (false, true, previous_ban_duration @ Some(_)) => {
			Ok(reason.duration(previous_ban_duration.map(Duration::from_secs)))
		}

		// The player isn't currently banned, has never been banned, but has a
		// total ban duration...?
		(false, false, Some(_)) => {
			unreachable!("cannot have ban duration without bans");
		}

		// The player is currently banned, was never banned in the past, but has
		// a previous ban duration?
		(true, false, Some(_)) => {
			unreachable!("cannot be currently banned with 0 previous bans");
		}

		// The player is not currently banned, was banned in the past, but has no
		// total ban duration...?
		(false, true, None) => {
			unreachable!("cannot be not-banned and have perma ban at the same time");
		}

		// Player is currently banned, so we can't ban them again
		(true, ..) => Err(Error::PlayerAlreadyBanned { steam_id: player_id }),
	}
}

/// Resolves a player's IP address by mapping IPv4 to IPv6 or fetching the
/// missing IP from the database.
async fn resolve_player_ip(
	player_ip: Option<IpAddr>,
	player_id: SteamID,
	txn: &mut Transaction<'_, MySql>,
) -> Result<Ipv6Addr>
{
	Ok(match player_ip {
		Some(ip) => ip.to_v6(),
		None => sqlx::query_scalar! {
			r"
			SELECT
			  ip_address `ip: Ipv6Addr`
			FROM
			  Players
			WHERE
			  id = ?
			LIMIT
			  1
			",
			player_id,
		}
		.fetch_optional(txn.as_mut())
		.await?
		.ok_or(Error::PlayerDoesNotExist { steam_id: player_id })?,
	})
}

#[allow(clippy::missing_docs_in_private_items)]
struct BannedByDetails
{
	server_id: Option<ServerID>,
	admin_id: Option<SteamID>,
	plugin_version_id: PluginVersionID,
}

/// Extracts the relevant details out of a [`BannedBy`] and fetches additional
/// information from the database.
async fn banned_by_details(
	banned_by: BannedBy,
	txn: &mut Transaction<'_, MySql>,
) -> Result<BannedByDetails>
{
	Ok(match banned_by {
		BannedBy::Server { id, plugin_version_id } => {
			BannedByDetails { server_id: Some(id), admin_id: None, plugin_version_id }
		}
		BannedBy::Admin { steam_id } => BannedByDetails {
			server_id: None,
			admin_id: Some(steam_id),
			plugin_version_id: sqlx::query_scalar! {
				r"
				SELECT
				  id `id: PluginVersionID`
				FROM
				  PluginVersions
				ORDER BY
				  created_on DESC
				LIMIT
				  1
				",
			}
			.fetch_one(txn.as_mut())
			.await?,
		},
	})
}

/// Creates a new ban in the database and returns its ID.
async fn create_ban(
	player_id: SteamID,
	player_ip: Ipv6Addr,
	reason: BanReason,
	banned_by_details: &BannedByDetails,
	ban_duration: Duration,
	txn: &mut Transaction<'_, MySql>,
) -> Result<BanID>
{
	Ok(sqlx::query! {
		r"
		INSERT INTO
		  Bans(
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
		",
		player_id,
		player_ip,
		banned_by_details.server_id,
		reason,
		banned_by_details.admin_id,
		banned_by_details.plugin_version_id,
		Utc::now() + ban_duration,
	}
	.execute(txn.as_mut())
	.await
	.map_err(|error| {
		if error.is_fk_violation("player_id") {
			Error::PlayerDoesNotExist { steam_id: player_id }
		} else if error.is_fk_violation("admin_id") {
			Error::PlayerDoesNotExist {
				steam_id: banned_by_details
					.admin_id
					.expect("we need a non-null admin_id to get this conflict"),
			}
		} else {
			Error::from(error)
		}
	})?
	.last_insert_id()
	.conv::<BanID>())
}
