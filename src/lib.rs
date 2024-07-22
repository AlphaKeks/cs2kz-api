#![doc = include_str!("../README.md")]
#![allow(warnings)] // FIXME

/*
 * CS2KZ API - the core infrastructure for CS2KZ.
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

use std::net::SocketAddr;

use sqlx::pool::PoolOptions;

#[macro_use]
extern crate pin_project;

// https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
#[macro_use]
extern crate sealed;

/// The server type that wraps the API.
pub type Server =
	axum::extract::connect_info::IntoMakeServiceWithConnectInfo<axum::Router, SocketAddr>;

pub mod setup;
pub mod runtime;

pub mod services;

mod middleware;
mod util;
mod database;
mod http;

/// Create a server that will run the API.
pub async fn server(config: runtime::Config) -> Result<Server, setup::Error>
{
	use self::services::{
		AdminService,
		AuthService,
		BanService,
		HealthService,
		JumpstatService,
		MapService,
		PlayerService,
		PluginService,
		RecordService,
		ServerService,
		SteamService,
	};

	let database = PoolOptions::new()
		.min_connections(database::MIN_CONNECTIONS)
		.max_connections(database::MAX_CONNECTIONS)
		.connect(config.database_url().as_str())
		.await?;

	sqlx::migrate!("./database/migrations")
		.run(&database)
		.await?;

	let http_client = reqwest::Client::new();

	let steam_svc = SteamService::new(config.clone(), http_client.clone());
	let auth_svc =
		AuthService::new(config.clone(), database.clone(), http_client.clone(), steam_svc.clone())?;

	let health_svc = HealthService::new();
	let player_svc = PlayerService::new();
	let map_svc = MapService::new(database.clone(), steam_svc.clone());
	let server_svc = ServerService::new(database.clone());
	let record_svc = RecordService::new();
	let jumpstat_svc = JumpstatService::new(database.clone(), auth_svc.clone());
	let ban_svc = BanService::new(database.clone(), auth_svc.clone());
	let admin_svc = AdminService::new(database.clone(), auth_svc.clone());
	let plugin_svc = PluginService::new();

	let logging = middleware::logging::layer!();
	let panic_handler = middleware::panic_handler::layer();

	let server = axum::Router::new()
		.merge(health_svc)
		.merge(player_svc)
		.merge(map_svc)
		.merge(server_svc)
		.merge(record_svc)
		.merge(jumpstat_svc)
		.merge(ban_svc)
		.merge(auth_svc)
		.merge(admin_svc)
		.merge(plugin_svc)
		.layer(logging)
		.layer(panic_handler)
		.into_make_service_with_connect_info::<SocketAddr>();

	Ok(server)
}
