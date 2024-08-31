use std::time::Duration;

use axum::{routing, Router};
use serde::Serialize;
use tokio::time::timeout;
use tokio::{runtime, task};
use tower_http::set_status::SetStatusLayer;
use tower_http::validate_request::ValidateRequestHeaderLayer;

use self::is_localhost::IsLocalhost;
use self::taskdump::Taskdump;

mod is_localhost;
mod taskdump;

pub fn router() -> Router
{
	let auth = (
		SetStatusLayer::new(http::StatusCode::NOT_FOUND),
		ValidateRequestHeaderLayer::custom(IsLocalhost),
	);

	let router = Router::new().route("/taskdump", routing::get(taskdump));

	router.layer(auth)
}

async fn taskdump() -> Result<Vec<u8>, taskdump::Error>
{
	let runtime = runtime::Handle::current();
	let dumps = timeout(Duration::from_secs(3), runtime.dump()).await?;
	let mut tasks = dumps.tasks().iter();

	tasks
		.try_fold(vec![b'['], |mut bytes, dump| {
			if !matches!(bytes.last(), Some(b'[')) {
				bytes.push(b',');
			}

			Taskdump::new(dump).serialize(&mut serde_json::Serializer::new(&mut bytes))?;

			Ok(bytes)
		})
		.map(|mut bytes| {
			bytes.push(b']');
			bytes
		})
}
