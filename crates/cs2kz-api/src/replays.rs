use axum::extract::{FromRef, State};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};
use cs2kz::Context;
use uuid::Uuid;

use crate::extract::Path;
use crate::response::ErrorResponse;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    Router::new().route("/{replay_id}", routing::get(get_replay))
}

#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/replays/{replay_id}",
    tag = "Replays",
    params(("replay_id" = Uuid,)),
    responses(
        (status = 200,),
        (status = 400, description = "invalid query parameters"),
        (status = 404,),
    ),
)]
async fn get_replay(
    State(cx): State<Context>,
    Path(replay_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let Some(replay_bucket) = cx.replay_bucket() else {
        tracing::debug!("replay storage not configured");
        return Err(ErrorResponse::not_found());
    };

    let res = replay_bucket
        .get_object_stream(replay_id.to_string())
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?;

    if res.status_code != 200 {
        tracing::debug!("failed to fetch replay");
        return Err(ErrorResponse::not_found());
    }

    let stream = tokio_util::io::ReaderStream::new(res);

    Ok(axum::body::Body::from_stream(stream).into_response())
}
