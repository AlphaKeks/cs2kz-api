//! A service for managing KZ admins.

use axum::extract::FromRef;
use cs2kz::SteamID;
use sqlx::{MySql, Pool};

use crate::database::TransactionExt;
use crate::services::auth::session::user;
use crate::services::AuthService;

mod error;
pub use error::{Error, Result};

mod models;
pub use models::{
	FetchAdminRequest,
	FetchAdminResponse,
	FetchAdminsRequest,
	FetchAdminsResponse,
	SetPermissionsRequest,
};

mod http;

/// A service for managing KZ admins.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct AdminService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl AdminService
{
	/// Create a new [`AdminService`].
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetches an admin by their SteamID.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_admin(&self, req: FetchAdminRequest) -> Result<Option<FetchAdminResponse>>
	{
		sqlx::query_as! {
			FetchAdminResponse,
			r"
			SELECT
			  name,
			  id `steam_id: SteamID`,
			  permissions `permissions: user::Permissions`
			FROM
			  Players
			WHERE
			  id = ?
			  AND permissions > 0
			",
			req.user_id,
		}
		.fetch_optional(&self.database)
		.await
		.map_err(Into::into)
	}

	/// Fetches many admins.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_admins(&self, req: FetchAdminsRequest) -> Result<FetchAdminsResponse>
	{
		let mut txn = self.database.begin().await?;
		let admins = sqlx::query_as! {
			FetchAdminResponse,
			r"
			SELECT
			  SQL_CALC_FOUND_ROWS name,
			  id `steam_id: SteamID`,
			  permissions `permissions: user::Permissions`
			FROM
			  Players
			WHERE
			  permissions > 0
			  AND ((permissions & ?) = ?)
			LIMIT
			  ? OFFSET ?
			",
			req.required_permissions,
			req.required_permissions,
			*req.limit,
			*req.offset,
		}
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchAdminsResponse { admins, total })
	}

	/// Set a user's permissions.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn set_permissions(&self, req: SetPermissionsRequest) -> Result<()>
	{
		let mut txn = self.database.begin().await?;

		let query_result = sqlx::query! {
			r"
			UPDATE
			  Players
			SET
			  permissions = ?
			WHERE
			  id = ?
			",
			req.permissions,
			req.user_id,
		}
		.execute(txn.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::UserDoesNotExist { user_id: req.user_id }),
			n => assert_eq!(n, 1, "updated more than 1 user"),
		}

		txn.commit().await?;

		tracing::info! {
			target: "cs2kz_api::audit_log",
			user_id = %req.user_id,
			permissions = %req.permissions,
			"set permissions for user",
		};

		Ok(())
	}
}
