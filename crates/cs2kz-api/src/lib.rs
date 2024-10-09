// This crate is part of the cs2kz-api project.
//
// Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see https://www.gnu.org/licenses.

#[macro_use]
extern crate derive_more;

#[macro_use]
extern crate thiserror;

#[allow(unused_imports, reason = "may be used later")]
#[macro_use(
	error, error_span, warn, warn_span, info, info_span, debug, debug_span, trace, trace_span,
	instrument
)]
extern crate tracing;

// Some proc-macros refer to `serde_json`, so we can't rename it in cargo.
extern crate serde_json as json;

#[macro_use]
pub(crate) mod macros;

#[cfg(test)]
pub(crate) mod testing;

pub(crate) mod serde;
pub(crate) mod database;
pub(crate) mod problem_details;
pub(crate) mod git;
pub(crate) mod events;

pub mod users;
pub mod plugin_versions;
pub mod players;
pub mod servers;
pub mod bans;
pub mod maps;
pub mod records;
pub mod jumpstats;
pub mod game_sessions;
