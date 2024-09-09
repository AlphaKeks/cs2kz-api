//! The CS2KZ API.
//!
//! This crate implements the core business logic, as well as an HTTP facade.

/*
 * CS2KZ API
 *
 * Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see https://www.gnu.org/licenses.
 */

use std::sync::Arc;
use std::time::Duration;
use std::{env, io};

use tokio_util::sync::CancellationToken;
use tower_sessions::CookieOptions;

#[macro_use]
extern crate tracing as _;

#[macro_use]
extern crate pin_project as _;

#[macro_use]
extern crate thiserror as _;

#[cfg(test)]
mod testing;

#[macro_use]
mod macros;

mod auth;
mod database;
mod extract;
mod http;
mod internal;
mod middleware;
mod signals;
mod tracing;
mod util;

pub mod config;
pub use config::Config;

pub mod services;
pub mod stats;

/// Runs the HTTP server.
pub async fn run(config: Config) -> Result<(), RunError>
{
	http::problem::set_base_uri(
		"https://docs.cs2kz.org/api/problems"
			.parse()
			.expect("valid uri"),
	);

	let shutdown_token = CancellationToken::new();
	let tracing_guard = tracing::init(&config.tracing)?;
	let database_url = env::var("DATABASE_URL")?.parse::<url::Url>()?;
	let pool = database::connect(
		&database_url,
		config.database.min_connections,
		config.database.max_connections,
	)
	.await?;

	let console_addr =
		config
			.tracing
			.console
			.as_ref()
			.and_then(|config| match config.server_addr {
				console_subscriber::ServerAddr::Tcp(addr) => Some(addr),
				_ => None,
			});

	let http_client = reqwest::Client::new();
	let steam_api_key = Arc::from(config.steam.api_key);
	let workshop_asset_dir = Arc::from(config.steam.workshop_asset_dir);
	let depot_downloader = Arc::from(config.steam.depot_downloader);
	let cookie_options = CookieOptions::new(String::from(config.http.cookie_domain), "/")
		.secure(cfg!(feature = "production"));

	let session_store = auth::SessionStore::new(pool.clone());

	let steam_service = services::SteamService::new(
		steam_api_key,
		http_client,
		workshop_asset_dir,
		depot_downloader,
	);

	let plugin_service = services::PluginService::new(pool.clone());
	let plugin_router = services::plugin::http::router(plugin_service);

	let player_service = services::PlayerService::new(pool.clone(), steam_service.clone());
	let player_router = services::players::http::router(player_service.clone());

	let map_service = services::MapService::new(pool.clone(), steam_service);
	let map_router = services::maps::http::router(map_service);

	let server_service = services::ServerService::new(pool.clone());
	let (ws_tasks, server_router) = services::servers::http::router(
		server_service,
		Arc::new(cookie_options),
		session_store,
		config.http.websocket_heartbeat_interval.into(),
		shutdown_token.child_token(),
	);

	let service = axum::Router::new()
		.route("/", axum::routing::get(|| async { "(͡ ͡° ͜ つ ͡͡°)" }))
		.nest("/_internal", internal::router(console_addr))
		.nest("/plugin", plugin_router)
		.nest("/players", player_router)
		.nest("/servers", server_router)
		.nest("/maps", map_router)
		.layer(middleware::catch_panic::layer())
		.layer(middleware::trace::layer())
		.layer(middleware::request_id::layer())
		.into_make_service_with_connect_info::<std::net::SocketAddr>();

	let tcp_listener = tokio::net::TcpListener::bind(config.http.listen_on).await?;
	let addr = tcp_listener.local_addr()?;

	info!(%addr, "listening for http requests");

	axum::serve(tcp_listener, service)
		.with_graceful_shutdown(signals::sigint())
		.await?;

	warn!("telling websockets to shut down");

	if !ws_tasks.close() {
		warn!("tracker already closed?");
	}

	shutdown_token.cancel();

	if tokio::time::timeout(Duration::from_secs(5), ws_tasks.wait())
		.await
		.is_err()
	{
		warn!("websockets did not shut down within 5 seconds");
	}

	warn!("closing database connections");
	pool.close().await;

	drop(tracing_guard);

	Ok(())
}

/// Errors returned by [`run()`].
#[derive(Debug, thiserror::Error)]
pub enum RunError
{
	/// We failed to read the `DATABASE_URL` environment variable.
	#[error("failed to get `DATABASE_URL` environment variable: {0}")]
	GetDatabaseUrl(#[from] env::VarError),

	/// We failed to parse the `DATABASE_URL` environment variable as a URL.
	#[error("failed to parse `DATABASE_URL` as a URL: {0}")]
	ParseDatabaseUrl(#[from] url::ParseError),

	/// We failed to establish a database connection.
	#[error("failed to establish database connection: {0}")]
	EstablishDatabaseConnection(#[from] sqlx::Error),

	/// Some other I/O failure.
	#[error(transparent)]
	Io(#[from] io::Error),
}
