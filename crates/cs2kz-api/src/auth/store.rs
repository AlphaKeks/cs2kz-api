use cookie::Expiration;
use cs2kz::SteamID;
use derive_more::{Constructor, Debug};
use time::Duration;

use super::{Error, Permissions, Result, Session, SessionData, SessionID};
use crate::database;
use crate::util::net::IpAddr;
use crate::util::time::Timestamp;

#[derive(Debug, Clone, Constructor)]
pub struct SessionStore
{
	#[debug("MySql")]
	mysql: database::Pool,
}

impl SessionStore
{
	pub async fn login(
		&self,
		user_id: SteamID,
		user_name: &str,
		user_ip: IpAddr,
	) -> Result<SessionID>
	{
		let session_id = SessionID::new();
		let expires_on = expiration();
		let mut txn = self.mysql.begin().await?;

		if sqlx::query! {
			"INSERT INTO Users
			   (id, name, ip_address)
			 VALUES
			   (?, ?, ?)",
			user_id,
			user_name,
			user_ip,
		}
		.execute(txn.as_mut())
		.await
		.is_ok()
		{
			debug!("user did not exist; created account");
		}

		sqlx::query! {
			"INSERT INTO UserSessions
			   (id, user_id, expires_on)
			 VALUES
			   (?, ?, ?)",
			session_id,
			user_id,
			expires_on,
		}
		.execute(txn.as_mut())
		.await?;

		info!(id = %session_id, "created session");

		txn.commit().await?;

		Ok(session_id)
	}

	pub async fn invalidate_all_sessions(&self, user_id: SteamID) -> Result<()>
	{
		sqlx::query! {
			"UPDATE UserSessions
			 SET expires_on = NOW()
			 WHERE user_id = ?",
			user_id,
		}
		.execute(&self.mysql)
		.await?;

		Ok(())
	}
}

impl tower_sessions::SessionStore for SessionStore
{
	type ID = SessionID;
	type Data = SessionData;
	type Error = Error;

	async fn load_session(&mut self, session_id: &SessionID) -> Result<SessionData>
	{
		let data = sqlx::query_as! {
			SessionData,
			"SELECT
			   u.id `user_id: SteamID`,
			   u.permissions `permissions: Permissions`,
			   s.expires_on `expires_on: Timestamp`
			 FROM UserSessions s
			 JOIN Users u ON u.id = s.user_id
			 WHERE s.id = ?",
			session_id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::UnknownSessionID)?;

		if data.has_expired() {
			return Err(Error::SessionExpired);
		}

		Ok(data)
	}

	async fn save_session(&mut self, session: Session) -> Result<Expiration>
	{
		let expiration = expiration();

		sqlx::query! {
			"UPDATE UserSessions
			 SET expires_on = ?
			 WHERE id = ?",
			expiration,
			session.id(),
		}
		.execute(&self.mysql)
		.await?;

		Ok(Expiration::DateTime(expiration))
	}

	async fn invalidate_session(&mut self, session: Session) -> Result<()>
	{
		sqlx::query! {
			"UPDATE UserSessions
			 SET expires_on = NOW()
			 WHERE id = ?",
			session.id(),
		}
		.execute(&self.mysql)
		.await?;

		Ok(())
	}
}

fn expiration() -> time::OffsetDateTime
{
	time::OffsetDateTime::now_utc() + (Duration::WEEK * 2)
}
