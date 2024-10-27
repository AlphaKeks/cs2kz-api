use axum::Router;
use axum::extract::FromRef;
use axum::handler::Handler;
use axum::routing::{self, MethodRouter};
use problem_details::AsProblemDetails;

use crate::config::Credentials;
use crate::http::extract::{Json, Path, Query};
use crate::http::middleware::api_key_auth::{ApiKeyAuthState, api_key_auth};
use crate::http::problem_details::Problem;
use crate::http::response::{Created, ErrorResponse, NotFound};
use crate::pagination::{Limit, Offset, PaginationResult, TryStreamExt as _};
use crate::plugin::{self, NewPluginVersion, PluginVersion, PluginVersionID};
use crate::{database, git};

pub fn router<S>(credentials: &Credentials, database: database::ConnectionPool) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	let auth = axum::middleware::from_fn_with_state(
		ApiKeyAuthState::new(&*credentials.publish_plugin_version, database),
		api_key_auth,
	);

	Router::new()
		.route(
			"/versions",
			MethodRouter::new()
				.get(get_plugin_versions)
				.post(publish_plugin_version.layer(auth)),
		)
		.route("/versions/{version}", routing::get(get_plugin_version))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
struct GetPluginVersionsQuery {
	#[serde(default)]
	limit: Limit<50, 10>,

	#[serde(default)]
	offset: Offset,
}

/// Returns metadata about previous cs2kz-metamod releases.
#[instrument]
#[utoipa::path(get, path = "/plugin/versions", tag = "Plugin", params(GetPluginVersionsQuery), responses(
	(status = 200, body = PaginationResult<PluginVersion>),
))]
pub(crate) async fn get_plugin_versions(
	mut db_conn: database::Connection,
	Query(GetPluginVersionsQuery { limit, offset }): Query<GetPluginVersionsQuery>,
) -> Result<PaginationResult<PluginVersion>, ErrorResponse> {
	let (total, stream) = plugin::get_versions(&mut db_conn, limit.get(), offset.0).await?;
	let result = stream
		.try_collect_into_pagination_result(total, limit.max())
		.await?;

	Ok(result)
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
struct CreatedPluginVersion {
	plugin_version_id: PluginVersionID,
}

/// Publishes a new release of cs2kz-metamod.
///
/// **NOTE**: this is for internal use
#[instrument]
#[utoipa::path(post, path = "/plugin/versions", tag = "Plugin", responses(
	(status = 201, body = CreatedPluginVersion),
	(status = 409, description = "the version either already exists or is older than the latest version"),
))]
pub(crate) async fn publish_plugin_version(
	mut db_conn: database::Connection,
	Json(plugin_version): Json<NewPluginVersion>,
) -> Result<Created<CreatedPluginVersion>, ErrorResponse> {
	let plugin_version_id = plugin::create_version(&mut db_conn, &plugin_version).await?;

	Ok(Created(CreatedPluginVersion { plugin_version_id }))
}

/// Returns metadata about a specific plugin version.
#[instrument]
#[utoipa::path(
	get,
	path = "/plugin/versions/{version}",
	tag = "Plugin",
	params(("version", description = "an ID, a SemVer identifier, or a git revision")),
	responses((status = 200, body = PluginVersion)),
)]
pub(crate) async fn get_plugin_version(
	mut db_conn: database::Connection,
	Path(version): Path<String>,
) -> Result<Json<PluginVersion>, ErrorResponse> {
	#[derive(Debug, Error)]
	#[error("unknown plugin version identifier")]
	struct UnknownPluginVersionIdentifier;

	impl AsProblemDetails for UnknownPluginVersionIdentifier {
		type ProblemType = Problem;

		fn problem_type(&self) -> Self::ProblemType {
			Problem::InvalidPathParameters
		}
	}

	let plugin_version = if let Ok(plugin_version_id) = version.parse::<PluginVersionID>() {
		plugin::get_version_by_id(&mut db_conn, plugin_version_id).await?
	} else if let Ok(git_revision) = version.parse::<git::Revision>() {
		plugin::get_version_by_git_revision(&mut db_conn, &git_revision).await?
	} else if let Ok(semver) = version.parse::<semver::Version>() {
		plugin::get_version_by_semver_ident(&mut db_conn, &semver).await?
	} else {
		return Err(UnknownPluginVersionIdentifier.into());
	}
	.ok_or(NotFound)?;

	Ok(Json(plugin_version))
}
