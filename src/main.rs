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

use std::backtrace::Backtrace;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, panic};

use anyhow::Context;
use clap::{Parser, Subcommand};
use similar::TextDiff;
use tracing::Instrument;

mod logging;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode>
{
	match CLI::parse().action.unwrap_or_default() {
		Action::Serve { env_file } => {
			if let Some(path) = env_file.as_deref() {
				dotenvy::from_filename(path).context("load custom `.env` file")?;
			} else if dotenvy::dotenv().is_err() {
				// `.env` files missing is not necessarily an issue (e.g. when running tests
				// in CI), but we log it to stderr just in case.
				eprintln!("WARNING: no `.env` file found");
			}
		}
		Action::GenerateSpec { check } => {
			return generate_spec(check.as_deref());
		}
	}

	let _guard = logging::init().context("initialize logging")?;
	let runtime_span = tracing::info_span!("runtime::startup");
	let api_config = runtime_span
		.in_scope(cs2kz_api::Config::new)
		.context("load config")?;

	let old_panic_hook = panic::take_hook();

	// If anything anywhere ever panics, we want to log it.
	panic::set_hook(Box::new(move |info| {
		tracing::error_span!("runtime::panic_hook").in_scope(|| {
			let backtrace = Backtrace::force_capture();
			tracing::error! {
				target: "cs2kz_api::audit_log",
				"{info}\n\nstack backtrace:\n{backtrace}",
			};
		});

		old_panic_hook(info)
	}));

	cs2kz_api::run(api_config)
		.instrument(runtime_span)
		.await
		.context("run API")?;

	Ok(ExitCode::SUCCESS)
}

/// CS2KZ API
#[derive(Debug, Parser)]
struct CLI
{
	/// What you want to do
	#[command(subcommand)]
	action: Option<Action>,
}

#[derive(Debug, Subcommand)]
enum Action
{
	/// Serve the API
	Serve
	{
		/// Use a custom `.env` file.
		#[arg(long, name = "FILE")]
		env_file: Option<PathBuf>,
	},

	/// Generate a JSON representation of the API's OpenAPI spec.
	GenerateSpec
	{
		/// Do not print the generated spec, only diff it against an existing
		/// one. This will exit with code 1 if any diffs are found.
		#[arg(long, name = "FILE")]
		check: Option<PathBuf>,
	},
}

impl Default for Action
{
	fn default() -> Self
	{
		Self::Serve { env_file: None }
	}
}

/// Serializes the API's OpenAPI spec as JSON and potentially diffs it against
/// an existing spec file.
///
/// If `check` is specified, the real spec will be diffed against the spec
/// stored at the specified path. Any diffs will be printed, and the exit status
/// will be 1 if any diffs are found.
///
/// Otherwise, the spec is simply printed to stdout.
fn generate_spec(check: Option<&Path>) -> anyhow::Result<ExitCode>
{
	let spec = cs2kz_api::openapi::Spec::new().as_json();

	let Some(path) = check else {
		print!("{spec}");
		return Ok(ExitCode::SUCCESS);
	};

	let file = fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
	let exit_code = TextDiff::from_lines(&file, &spec)
		.unified_diff()
		.iter_hunks()
		.fold(ExitCode::SUCCESS, |_, hunk| {
			eprintln!("{hunk}");
			ExitCode::FAILURE
		});

	Ok(exit_code)
}
