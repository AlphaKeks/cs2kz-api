//! This module contains the HTTP handlers for the `/plugin` endpoint.

use axum::extract::State;
use axum::{routing, Router};
use problem_details::AsProblemDetails;

use super::{get_version, get_versions, submit_version, PluginService, PluginVersionIdentifier};
use crate::extract::{Json, Path, Query};
use crate::http::Created;

mod auth;

/// Returns a router for the `/plugin` endpoint.
pub fn router(plugin_service: PluginService) -> Router
{
	let auth =
		axum::middleware::from_fn_with_state(plugin_service.mysql.clone(), auth::ensure_api_key);

	Router::new()
		.route("/versions", routing::get(get_versions))
		.route("/versions", routing::post(submit_version).route_layer(auth))
		.route("/versions/:version", routing::get(get_version))
		.with_state(plugin_service)
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_version(
	State(plugin_service): State<PluginService>,
	Path(version_identifier): Path<PluginVersionIdentifier>,
) -> crate::http::Result<Json<get_version::Response>>
{
	let version = match version_identifier {
		PluginVersionIdentifier::ID(id) => plugin_service.get_version_by_id(id).await,
		PluginVersionIdentifier::Name(name) => plugin_service.get_version_by_name(&name).await,
		PluginVersionIdentifier::Revision(revision) => {
			plugin_service.get_version_by_git_revision(&revision).await
		}
	}
	.map_err(|error| error.as_problem_details())?;

	Ok(Json(version))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_versions(
	State(plugin_service): State<PluginService>,
	Query(request): Query<get_versions::Request>,
) -> crate::http::Result<Json<get_versions::Response>>
{
	let versions = plugin_service
		.get_versions(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(versions))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn submit_version(
	State(plugin_service): State<PluginService>,
	Json(request): Json<submit_version::Request>,
) -> crate::http::Result<Created<submit_version::Response>>
{
	let response = plugin_service
		.submit_version(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	let location = location!("/plugin/versions/{}", response.version_id);

	Ok(Created::new(location, response))
}
