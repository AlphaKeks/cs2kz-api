//! Everything related to KZ admins.

use axum::extract::FromRef;
use cs2kz::SteamID;
use sqlx::{MySql, Pool};

use crate::authorization::Permissions;
use crate::sqlx::query;
use crate::{Error, Result};

mod models;
pub use models::{Admin, AdminUpdate, FetchAdminsRequest};

pub mod http;

/// A service for dealing with KZ admins as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct AdminService
{
	database: Pool<MySql>,
}

impl AdminService
{
	/// Creates a new [`AdminService`] instance.
	pub const fn new(database: Pool<MySql>) -> Self
	{
		Self { database }
	}

	/// Fetches a single admin.
	pub async fn fetch_admin(&self, admin_id: SteamID) -> Result<Admin>
	{
		let admin = sqlx::query! {
			r#"
			SELECT
			  id `id: SteamID`,
			  name,
			  permissions `permissions: Permissions`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			admin_id,
		}
		.fetch_optional(&self.database)
		.await?
		.map(|row| Admin { name: row.name, steam_id: row.id, permissions: row.permissions })
		.ok_or_else(|| Error::not_found("admin"))?;

		Ok(admin)
	}

	/// Fetches many admins.
	///
	/// The `limit` and `offset` fields in [`FetchAdminsRequest`] can be used
	/// for pagination. The `u64` part of the returned tuple indicates how many
	/// admins _could_ be fetched; also useful for pagination.
	pub async fn fetch_admins(&self, request: FetchAdminsRequest) -> Result<(Vec<Admin>, u64)>
	{
		let mut transaction = self.database.begin().await?;

		let admins = sqlx::query_as! {
			Admin,
			r#"
			SELECT SQL_CALC_FOUND_ROWS
			  id `steam_id: SteamID`,
			  name,
			  permissions `permissions: Permissions`
			FROM
			  Players
			WHERE
			  permissions > 0
			  AND (permissions & ?) = ?
			LIMIT
			  ? OFFSET ?
			"#,
			request.permissions,
			request.permissions,
			*request.limit,
			*request.offset,
		}
		.fetch_all(transaction.as_mut())
		.await?;

		if admins.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((admins, total))
	}

	/// Updates an existing admin.
	///
	/// This update is idempotent.
	/// Even though "admins" is usually a term used for players with
	/// permissions, this function will update any player; which means it can
	/// turn a non-admin into an admin, or turn an admin into a non-admin.
	pub async fn update_admin(&self, admin_id: SteamID, update: AdminUpdate) -> Result<()>
	{
		let mut transaction = self.database.begin().await?;

		let query_result = sqlx::query! {
			r#"
			UPDATE
			  Players
			SET
			  permissions = ?
			WHERE
			  id = ?
			"#,
			update.permissions,
			admin_id,
		}
		.execute(transaction.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("admin")),
			n => assert_eq!(n, 1, "updated more than 1 admin"),
		}

		transaction.commit().await?;

		tracing::trace! {
			target: "cs2kz_api::audit_log",
			%admin_id,
			permissions = ?update.permissions,
			"updated admin",
		};

		Ok(())
	}
}
