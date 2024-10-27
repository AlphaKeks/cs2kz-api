//! This crate is part of the cs2kz-api project.
//!
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

use std::process;

/* NOTE: We don't return a `Result` from `main()` because that would print
 *       errors using their `Debug` representation. We want human-readable
 *       error messages, so we manually print errors with `eprintln!` and
 *       return an exit code.
 */

fn main() -> process::ExitCode {
	match cs2kz_api::server::run() {
		Ok(()) => process::ExitCode::SUCCESS,

		#[expect(clippy::print_stderr)]
		Err(error) => {
			eprintln!("{error}");
			process::ExitCode::FAILURE
		},
	}
}
