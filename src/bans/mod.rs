pub mod banned_by;
mod id;
mod reason;
mod unban_reason;

use std::time::Duration;

use futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt};
use serde::Serialize;
use sqlx::Row;
use utoipa::ToSchema;

pub use self::{
	banned_by::BannedBy,
	id::{BanId, ParseBanIdError},
	reason::BanReason,
	unban_reason::{InvalidUnbanReason, UnbanReason},
};
use crate::{
	database::{DatabaseConnection, DatabaseError, DatabaseResult},
	players::{PlayerId, PlayerIp},
	stream::StreamExt as _,
	time::Timestamp,
	users::UserId,
};

#[derive(Debug)]
pub struct CreatedBan
{
	pub id: BanId,
	pub expires_at: Timestamp,
}

#[derive(Debug, Display, Error, From)]
pub enum CreateBanError
{
	UnknownPlayer,
	AlreadyBanned,

	#[from(DatabaseError, sqlx::Error)]
	DatabaseError(DatabaseError),
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	player_ip: Option<PlayerIp>,
	reason: BanReason,
	#[builder(into)] banned_by: BannedBy,
	#[builder(into)] expires_after: Option<Duration>,
) -> Result<CreatedBan, CreateBanError>
{
	let expires_after = if let Some(duration) = expires_after {
		duration
	} else {
		let previous_ban_duration = sqlx::query_scalar!(
			"SELECT SUM(expires_at - created_at) AS `ban_duration: Duration`
             FROM Bans
             WHERE player_id = ?
             GROUP BY player_id",
			player_id,
		)
		.fetch(conn.as_raw())
		.map_ok(Option::unwrap_or_default)
		.try_fold(Duration::ZERO, async |total, duration| Ok(total + duration))
		.await?;

		reason.ban_duration(previous_ban_duration)
	};

	let player_ip = if let Some(ip) = player_ip {
		ip
	} else {
		sqlx::query_scalar!(
			"SELECT ip_address AS `ip_address: PlayerIp`
			 FROM Players
			 WHERE id = ?",
			player_id,
		)
		.fetch_optional(conn.as_raw())
		.await?
		.ok_or(CreateBanError::UnknownPlayer)?
	};

	let expires_at = Timestamp::now() + expires_after;
	let ban_id = sqlx::query!(
		"INSERT INTO Bans (player_id, player_ip, reason, banned_by, expires_at)
		 VALUES (?, ?, ?, ?, ?)
		 RETURNING id",
		player_id,
		player_ip,
		reason,
		banned_by,
		expires_at,
	)
	.fetch_one(conn.as_raw())
	.and_then(async |row| row.try_get(0))
	.await?;

	Ok(CreatedBan { id: ban_id, expires_at })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Ban
{
	pub id: BanId,
	pub player_id: PlayerId,
	pub reason: BanReason,
	pub banned_by: BannedBy,
	pub created_at: Timestamp,
	pub expires_at: Timestamp,
	pub unban: Option<Unban>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Unban
{
	pub reason: UnbanReason,
	pub unbanned_by: UserId,
	pub created_at: Timestamp,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	player: Option<PlayerId>,
	#[builder(into)] banned_by: Option<BannedBy>,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(*)
		 FROM Bans
		 WHERE player_id = COALESCE(?, player_id)
		 AND banned_by = COALESCE(?, banned_by)",
		player,
		banned_by,
	)
	.fetch_one(conn.as_raw())
	.map_err(DatabaseError::from)
	.and_then(async |row| row.try_into().map_err(DatabaseError::convert_count))
	.await
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	player: Option<PlayerId>,
	#[builder(into)] banned_by: Option<BannedBy>,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Ban>>
{
	sqlx::query!(
		"SELECT
		   b.id AS `id: BanId`,
		   b.player_id AS `player_id: PlayerId`,
		   b.reason AS `reason: BanReason`,
		   b.banned_by AS `banned_by: BannedBy`,
		   b.created_at AS `created_at: Timestamp`,
		   b.expires_at AS `expires_at: Timestamp`,
		   ub.reason AS `unban_reason: UnbanReason`,
		   ub.unbanned_by AS `unbanned_by: UserId`,
		   ub.created_at AS `unban_created_at: Timestamp`
		 FROM Bans AS b
		 LEFT JOIN Unbans AS ub ON ub.id = b.id
		 WHERE b.player_id = COALESCE(?, b.player_id)
		 AND b.banned_by = COALESCE(?, b.banned_by)
		 ORDER BY b.id DESC
		 LIMIT ?, ?",
		player,
		banned_by,
		offset,
		limit,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
	.and_then(async |row| {
		let unban = row
			.unban_reason
			.map(|reason| {
				let unbanned_by = row.unbanned_by.ok_or_else(|| {
					DatabaseError::decode_column(
						"unbanned_by",
						"got unban reason but missing unbanned_by",
					)
				})?;

				let created_at = row.unban_created_at.ok_or_else(|| {
					DatabaseError::decode_column(
						"unban_created_at",
						"got unban reason and unbanned_by but missing unban_created_at",
					)
				})?;

				DatabaseResult::Ok(Unban { reason, unbanned_by, created_at })
			})
			.transpose()?;

		Ok(Ban {
			id: row.id,
			player_id: row.player_id,
			reason: row.reason,
			banned_by: row.banned_by,
			created_at: row.created_at,
			expires_at: row.expires_at,
			unban,
		})
	})
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] ban_id: BanId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Ban>>
{
	sqlx::query!(
		"SELECT
		   b.id AS `id: BanId`,
		   b.player_id AS `player_id: PlayerId`,
		   b.reason AS `reason: BanReason`,
		   b.banned_by AS `banned_by: BannedBy`,
		   b.created_at AS `created_at: Timestamp`,
		   b.expires_at AS `expires_at: Timestamp`,
		   ub.reason AS `unban_reason: UnbanReason`,
		   ub.unbanned_by AS `unbanned_by: UserId`,
		   ub.created_at AS `unban_created_at: Timestamp`
		 FROM Bans AS b
		 LEFT JOIN Unbans AS ub ON ub.id = b.id
		 WHERE b.id = ?",
		ban_id,
	)
	.fetch_optional(conn.as_raw())
	.map_err(DatabaseError::from)
	.and_then(async |row| {
		let Some(row) = row else {
			return Ok(None);
		};

		let unban = row
			.unban_reason
			.map(|reason| {
				let unbanned_by = row.unbanned_by.ok_or_else(|| {
					DatabaseError::decode_column(
						"unbanned_by",
						"got unban reason but missing unbanned_by",
					)
				})?;

				let created_at = row.unban_created_at.ok_or_else(|| {
					DatabaseError::decode_column(
						"unban_created_at",
						"got unban reason and unbanned_by but missing unban_created_at",
					)
				})?;

				DatabaseResult::Ok(Unban { reason, unbanned_by, created_at })
			})
			.transpose()?;

		Ok(Some(Ban {
			id: row.id,
			player_id: row.player_id,
			reason: row.reason,
			banned_by: row.banned_by,
			created_at: row.created_at,
			expires_at: row.expires_at,
			unban,
		}))
	})
	.await
}

#[derive(Debug, Display, Error, From)]
pub enum UpdateBanError
{
	#[display("would expire in the past")]
	ExpiresInThePast,

	#[from(DatabaseError, sqlx::Error)]
	DatabaseError(DatabaseError),
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn update(
	#[builder(start_fn)] ban_id: BanId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	reason: Option<BanReason>,
	#[builder(into)] expires_after: Option<Duration>,
) -> Result<bool, UpdateBanError>
{
	let expires_at = match sqlx::query_scalar!(
		"SELECT created_at AS `created_at: Timestamp`
		 FROM Bans
		 WHERE id = ?",
		ban_id,
	)
	.fetch_optional(conn.as_raw())
	.await?
	{
		None => return Ok(false),
		Some(created_at) => expires_after.map(|duration| created_at + duration),
	};

	if expires_at.is_some_and(|timestamp| timestamp <= Timestamp::now()) {
		return Err(UpdateBanError::ExpiresInThePast);
	}

	sqlx::query!(
		"UPDATE Bans
		 SET reason = COALESCE(?, reason),
		     expires_at = COALESCE(?, expires_at)
		 WHERE id = ?",
		reason,
		expires_at,
		ban_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(UpdateBanError::from)
	.await
}

#[derive(Debug, Display, Error, From)]
pub enum RevertBanError
{
	#[display("ban is already expired")]
	AlreadyExpired,

	#[display("ban has already been reverted")]
	AlreadyUnbanned,

	#[display("invalid ban ID")]
	InvalidBanId,

	#[from(DatabaseError, sqlx::Error)]
	Database(DatabaseError),
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn revert(
	#[builder(start_fn)] ban_id: BanId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	reason: UnbanReason,
	unbanned_by: UserId,
) -> Result<(), RevertBanError>
{
	let has_expired = sqlx::query_scalar!("SELECT expires_at FROM Bans WHERE id = ?", ban_id)
		.fetch_optional(conn.as_raw())
		.await?
		.is_some_and(|expires_at| expires_at <= Timestamp::now());

	if has_expired {
		return Err(RevertBanError::AlreadyExpired);
	}

	let has_been_reverted = sqlx::query_scalar!(
		"SELECT (COUNT(*) > 0) AS `has_unbans: bool`
		 FROM Unbans
		 WHERE id = ?",
		ban_id,
	)
	.fetch_one(conn.as_raw())
	.await?;

	if has_been_reverted {
		return Err(RevertBanError::AlreadyUnbanned);
	}

	sqlx::query!(
		"INSERT INTO Unbans (id, reason, unbanned_by)
		 VALUES (?, ?, ?)",
		ban_id,
		reason,
		unbanned_by,
	)
	.execute(conn.as_raw())
	.await?;

	sqlx::query!("UPDATE Bans SET expires_at = NOW() WHERE id = ?", ban_id)
		.execute(conn.as_raw())
		.await?;

	Ok(())
}
