//! Handlers for the `/servers` route.

use std::net::Ipv4Addr;

use axum::Json;
use axum_extra::extract::Query;
use chrono::{DateTime, Utc};
use cs2kz::PlayerIdentifier;
use serde::Deserialize;
use tracing::info;
use utoipa::IntoParams;

use crate::authorization::Permissions;
use crate::database::{query, FilteredQuery, QueryBuilderExt, ResolveID, SqlxErrorExt};
use crate::http::{HandlerError, HandlerResult, Pagination};
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses::{self, Created};
use crate::servers::{queries, CreatedServer, NewServer, Server, ServerID};
use crate::{authentication, authorization, State};

/// Query parameters for `GET /servers`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by server name.
	name: Option<String>,

	/// Filter by IP address.
	#[param(value_type = Option<String>)]
	ip_address: Option<Ipv4Addr>,

	/// Filter by server owner.
	#[param(value_type = Option<parameters::PlayerIdentifier>)]
	owned_by: Option<PlayerIdentifier>,

	/// Only include servers approved after this date.
	created_after: Option<DateTime<Utc>>,

	/// Only include servers approved before this date.
	created_before: Option<DateTime<Utc>>,

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Fetch servers.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Servers",
  path = "/servers",
  params(GetParams),
  responses(
    responses::Ok<Pagination<Server>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: &State,
	Query(GetParams {
		name,
		ip_address,
		owned_by,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> HandlerResult<Json<Pagination<Server>>> {
	let mut transaction = state.database.begin().await?;
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(name) = name {
		query.filter("s.name LIKE ", format!("%{name}%"));
	}

	if let Some(ip_address) = ip_address {
		query.filter("s.ip_address = ", ip_address.to_string());
	}

	if let Some(owned_by) = owned_by {
		let owner_id = owned_by.resolve_id(transaction.as_mut()).await?;

		query.filter("o.id = ", owner_id);
	}

	if let Some(created_after) = created_after {
		query.filter("s.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter("s.created_on < ", created_before);
	}

	query.push_limits(limit, offset);

	let servers = query
		.build_query_as::<Server>()
		.fetch_all(transaction.as_mut())
		.await?;

	if servers.is_empty() {
		return Err(HandlerError::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(Pagination::new(total, servers)))
}

/// Approve a CS2 server.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  tag = "Servers",
  path = "/servers",
  request_body = NewServer,
  responses(
    responses::Created<CreatedServer>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn post(
	state: &State,
	session: authentication::Session<
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
	>,
	Json(NewServer {
		name,
		ip_address,
		owned_by,
	}): Json<NewServer>,
) -> HandlerResult<Created<Json<CreatedServer>>> {
	let mut transaction = state.database.begin().await?;
	let key = authentication::server::Key::new();
	let server_id = sqlx::query! {
		r#"
		INSERT INTO
		  Servers (name, ip_address, port, owned_by, `key`)
		VALUES
		  (?, ?, ?, ?, ?)
		"#,
		name,
		ip_address.ip().to_string(),
		ip_address.port(),
		owned_by,
		key,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation("owned_by") {
			HandlerError::unknown("server owner").with_source(err)
		} else {
			HandlerError::from(err)
		}
	})?
	.last_insert_id()
	.try_into()
	.map(ServerID)
	.expect("valid server ID");

	transaction.commit().await?;

	info!(target: "audit_log", %server_id, %key, admin = ?session.user(), "approved server");

	Ok(Created(Json(CreatedServer { server_id, key })))
}
