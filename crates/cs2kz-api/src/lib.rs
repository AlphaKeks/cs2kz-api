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

#![allow(unused)]

use std::{env, io};

#[macro_use]
extern crate tracing as _;

#[macro_use]
extern crate pin_project as _;

#[cfg(test)]
mod testing;

#[macro_use]
mod macros;

mod database;
mod http;
mod internal;
mod middleware;
mod signals;
mod tracing;
mod util;

pub mod config;
pub use config::Config;

/// Runs the HTTP server.
pub async fn run(config: Config) -> Result<(), RunError>
{
	http::problem::set_base_uri(
		"https://docs.cs2kz.org/api/problems"
			.parse()
			.expect("valid uri"),
	);

	let tracing_guard = tracing::init(&config.tracing)?;
	let database_url = env::var("DATABASE_URL")?.parse::<url::Url>()?;
	let pool = database::connect(
		&database_url,
		config.database.min_connections,
		config.database.max_connections,
	)
	.await?;

	let service = axum::Router::new()
		.route("/", axum::routing::get(|| async { "(͡ ͡° ͜ つ ͡͡°)" }))
		.nest("/_internal", internal::router())
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

	pool.close().await;
	drop(tracing_guard);

	Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RunError
{
	#[error("failed to get `DATABASE_URL` environment variable: {0}")]
	GetDatabaseUrl(#[from] env::VarError),

	#[error("failed to parse `DATABASE_URL` as a URL: {0}")]
	ParseDatabaseUrl(#[from] url::ParseError),

	#[error("failed to establish database connection: {0}")]
	EstablishDatabaseConnection(#[from] sqlx::Error),

	#[error(transparent)]
	Io(#[from] io::Error),
}
