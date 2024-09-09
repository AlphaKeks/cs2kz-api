use std::net::SocketAddr;
use std::time::Duration;

use axum::response::Redirect;
use axum::{routing, Router};
use axum_extra::middleware::option_layer;
use serde::Serialize;
use tokio::runtime;
use tokio::time::timeout;
use tower_http::add_extension::AddExtensionLayer;
use tower_http::set_status::SetStatusLayer;
use tower_http::validate_request::ValidateRequestHeaderLayer;

use self::is_localhost::IsLocalhost;
use self::taskdump::Taskdump;
use crate::extract::Extension;

mod is_localhost;
mod taskdump;

#[derive(Debug, Clone, Copy)]
struct ConsoleAddr(SocketAddr);

pub fn router(console_addr: Option<SocketAddr>) -> Router
{
	let auth = (
		SetStatusLayer::new(http::StatusCode::NOT_FOUND),
		ValidateRequestHeaderLayer::custom(IsLocalhost),
	);

	let console_addr = console_addr.map(ConsoleAddr).map(AddExtensionLayer::new);

	Router::new()
		.route(
			"/console",
			routing::get(console).route_layer(option_layer(console_addr)),
		)
		.route("/taskdump", routing::get(taskdump).route_layer(auth))
}

async fn console(console_addr: Option<Extension<ConsoleAddr>>)
	-> Result<Redirect, http::StatusCode>
{
	match console_addr {
		Some(Extension(ConsoleAddr(addr))) => Ok(Redirect::to(&format!("http://{addr}"))),
		None => Err(http::StatusCode::NOT_FOUND),
	}
}

async fn taskdump() -> Result<Vec<u8>, taskdump::Error>
{
	let runtime = runtime::Handle::current();
	let dump = timeout(Duration::from_secs(3), runtime.dump()).await?;
	let mut response = vec![b'['];

	for task in dump.tasks().iter() {
		if !matches!(response.last(), Some(b'[')) {
			response.push(b',');
		}

		Taskdump::new(task).serialize(&mut serde_json::Serializer::new(&mut response))?;
	}

	response.push(b']');

	Ok(response)
}
