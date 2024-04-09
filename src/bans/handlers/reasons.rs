//! Handlers for the `/bans/reasons` route.

use std::num::NonZeroU64;

use axum::extract::Path;
use axum::Json;
use sqlx::{MySqlExecutor, QueryBuilder};
use tracing::info;

use crate::auth::RoleFlags;
use crate::bans::{queries, Ban, BanUpdate, CreatedUnban, NewUnban};
use crate::responses::{Created, NoContent};
use crate::sqlx::extract::{Connection, Transaction};
use crate::sqlx::{query, UpdateQuery};
use crate::{auth, responses, Error, Result};

#[tracing::instrument(level = "debug")]
#[utoipa::path(
  get,
  path = "/bans/reasons",
  tag = "Bans",
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get() -> Result<Json<()>> {
	todo!();
}
