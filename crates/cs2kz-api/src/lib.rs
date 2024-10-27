/* CS2KZ API
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

//! The CS2KZ API.

#[macro_use(Error)]
extern crate thiserror;

#[allow(unused_imports, reason = "may be used later")]
#[macro_use(
	error, error_span, warn, warn_span, info, info_span, debug, debug_span, trace, trace_span,
	instrument
)]
extern crate tracing;

// We do this instead of renaming the crate with cargo because utoipa macros
// expect `serde_json` to be in scope.
extern crate serde_json as json;

#[macro_use]
mod macros;

pub mod cli;
pub mod config;
pub mod database;
pub mod http;
pub mod openapi;
pub mod players;
pub mod plugin;
pub mod server;
pub mod servers;
pub mod users;

mod git;
mod pagination;
mod state;
mod time;
