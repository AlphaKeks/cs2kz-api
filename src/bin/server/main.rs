//! CS2KZ API - the core infrastructure for CS2KZ.
//! Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program. If not, see https://www.gnu.org/licenses.

use tokio::net::TcpListener;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

/// The main server entrypoint for the API.
#[tokio::main]
async fn main() -> color_eyre::Result<()>
{
	color_eyre::install()?;
	cs2kz_api::runtime::panic_hook::install();
	tracing_subscriber::fmt()
		.pretty()
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	let tcp_listener = TcpListener::bind("0.0.0.0:42069").await?;
	let config = cs2kz_api::runtime::Config::new()?;
	let server = cs2kz_api::server(config).await?;

	tracing::info!("listening on {}", tcp_listener.local_addr()?);

	axum::serve(tcp_listener, server).await?;

	Ok(())
}
