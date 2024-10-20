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

#![doc = include_str!("../../../README.md")]

#[macro_use(Debug, Display, Deref, From, Into, FromStr)]
extern crate derive_more;

// I would prefer to rename this with cargo instead, but some naughty proc-macros assume a crate
// called `serde_json` is available, so what we need is an alias, not a rename.
extern crate serde_json as json;

#[macro_use(Error)]
extern crate thiserror;

#[allow(unused_imports, reason = "may be used later")]
#[macro_use(
	error, error_span, warn, warn_span, info, info_span, debug, debug_span, trace, trace_span,
	instrument
)]
extern crate tracing;

#[macro_use]
mod macros;

mod database;
mod email;
mod events;
mod http;
mod serde;
mod state;
mod time;

mod serve;
pub use serve::{serve, ServeError};

pub mod config;
pub mod openapi;

pub mod users;
