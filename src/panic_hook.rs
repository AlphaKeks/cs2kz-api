use std::{
	backtrace::{Backtrace, BacktraceStatus},
	panic,
};

use crate::runtime::{self, Environment};

pub(crate) fn install()
{
	panic::update_hook(|old_hook, panic_info| {
		let location = panic_info.location();
		let payload = panic_info.payload_as_str();
		let runtime_environment = runtime::environment::get();

		match (location, payload, runtime_environment) {
			(None, None, Environment::Development) => {
				tracing::error!(
					target: "cs2kz_api::panics",
					backtrace = %Backtrace::force_capture(),
					"thread panicked",
				);
			},
			(None, None, Environment::Testing | Environment::Production) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					tracing::error!(target: "cs2kz_api::panics", %backtrace, "thread panicked");
				} else {
					tracing::error!(target: "cs2kz_api::panics", "thread panicked");
				}
			},
			(None, Some(panic_message), Environment::Development) => {
				tracing::error!(
					target: "cs2kz_api::panics",
					backtrace = %Backtrace::force_capture(),
					"thread panicked: {panic_message}",
				);
			},
			(None, Some(panic_message), Environment::Testing | Environment::Production) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					tracing::error!(
						target: "cs2kz_api::panics",
						%backtrace,
						"thread panicked: {panic_message}",
					);
				} else {
					tracing::error!(
						target: "cs2kz_api::panics",
						"thread panicked: {panic_message}",
					);
				}
			},
			(Some(location), None, Environment::Development) => {
				tracing::error!(
					target: "cs2kz_api::panics",
					%location,
					backtrace = %Backtrace::force_capture(),
					"thread panicked",
				);
			},
			(Some(location), None, Environment::Testing | Environment::Production) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					tracing::error!(
						target: "cs2kz_api::panics",
						%location,
						%backtrace,
						"thread panicked",
					);
				} else {
					tracing::error!(
						target: "cs2kz_api::panics",
						%location,
						"thread panicked",
					);
				}
			},
			(Some(location), Some(panic_message), Environment::Development) => {
				tracing::error!(
					target: "cs2kz_api::panics",
					%location,
					backtrace = %Backtrace::force_capture(),
					"thread panicked: {panic_message}",
				);
			},
			(
				Some(location),
				Some(panic_message),
				Environment::Testing | Environment::Production,
			) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					tracing::error!(
						target: "cs2kz_api::panics",
						%location,
						%backtrace,
						"thread panicked: {panic_message}",
					);
				} else {
					tracing::error!(
						target: "cs2kz_api::panics",
						%location,
						"thread panicked: {panic_message}",
					);
				}
			},
		}

		old_hook(panic_info)
	});
}
