//! HTTP handlers for the `/plugin/versions` routes.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::authentication::ApiKey;
use crate::make_id::IntoID;
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses;
use crate::openapi::responses::{Created, PaginationResponse};
use crate::plugin::{CreatedPluginVersion, NewPluginVersion, PluginVersion, PluginVersionID};
use crate::sqlx::{query, QueryBuilderExt, SqlErrorExt};
use crate::{Error, Result, State};

/// Query parameters for `/plugin/versions`.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct GetParams {
	/// Maximum number of results to return.
	#[serde(default)]
	limit: Limit,

	/// Pagination offset.
	#[serde(default)]
	offset: Offset,
}

/// Fetch CS2KZ plugin versions.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/plugin/versions",
  tag = "CS2KZ Plugin",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<PluginVersion>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: State,
	Query(GetParams { limit, offset }): Query<GetParams>,
) -> Result<Json<PaginationResponse<PluginVersion>>> {
	let mut query = QueryBuilder::new("SELECT SQL_CALC_FOUND_ROWS * FROM PluginVersions");

	query.push_limits(limit, offset);

	let mut transaction = state.transaction().await?;

	let plugin_versions = query
		.build_query_as::<PluginVersion>()
		.fetch_all(transaction.as_mut())
		.await?;

	if plugin_versions.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: plugin_versions,
	}))
}

/// Submit a new CS2KZ plugin version.
///
/// This endpoint is intended to be used by GitHub Actions.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/plugin/versions",
  tag = "CS2KZ Plugin",
  security(("API Key" = ["plugin_versions"])),
  request_body = NewPluginVersion,
  responses(
    responses::Created<CreatedPluginVersion>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn post(
	state: State,
	api_key: ApiKey,
	Json(NewPluginVersion {
		semver,
		git_revision,
	}): Json<NewPluginVersion>,
) -> Result<Created<Json<CreatedPluginVersion>>> {
	if api_key.name() != "plugin_versions" {
		return Err(Error::unauthorized().context(api_key.to_string()));
	}

	let mut transaction = state.transaction().await?;

	let latest_version = sqlx::query! {
		r#"
		SELECT
		  semver
		FROM
		  PluginVersions
		ORDER BY
		  created_on DESC
		LIMIT
		  1
		"#
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.map(|row| row.semver.parse::<semver::Version>())
	.transpose()
	.map_err(|err| Error::logic("invalid semver in database").context(err))?;

	if let Some(version) = latest_version.filter(|version| version >= &semver) {
		tracing::warn! {
			target: "cs2kz_api::audit_log",
			latest = %version,
			actual = %semver,
			"submitted outdated plugin version",
		};

		return Err(Error::outdated_plugin_version(semver, version));
	}

	let plugin_version_id = sqlx::query! {
		r#"
		INSERT INTO
		  PluginVersions (semver, git_revision)
		VALUES
		  (?, ?)
		"#,
		semver.to_string(),
		git_revision,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_duplicate_entry() {
			Error::already_exists("plugin version").context(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into_id::<PluginVersionID>()?;

	transaction.commit().await?;

	tracing::debug! {
		target: "cs2kz_api::audit_log",
		id = %plugin_version_id,
		%semver,
		%git_revision,
		"created new plugin version",
	};

	Ok(Created(Json(CreatedPluginVersion { plugin_version_id })))
}
