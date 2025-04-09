use {
	crate::runtime::{self, Environment},
	std::{
		backtrace::{Backtrace, BacktraceStatus},
		panic,
	},
};

pub(crate) fn install()
{
	panic::update_hook(|old_hook, panic_info| {
		let location = panic_info.location();
		let payload = panic_info.payload_as_str();
		let runtime_environment = runtime::environment::get();

		match (location, payload, runtime_environment) {
			(None, None, Environment::Development) => {
				error!(
					target: "cs2kz_api::panics",
					backtrace = %Backtrace::force_capture(),
					"thread panicked",
				);
			},
			(None, None, Environment::Testing | Environment::Production) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					error!(target: "cs2kz_api::panics", %backtrace, "thread panicked");
				} else {
					error!(target: "cs2kz_api::panics", "thread panicked");
				}
			},
			(None, Some(panic_message), Environment::Development) => {
				error!(
					target: "cs2kz_api::panics",
					backtrace = %Backtrace::force_capture(),
					"thread panicked: {panic_message}",
				);
			},
			(None, Some(panic_message), Environment::Testing | Environment::Production) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					error!(
						target: "cs2kz_api::panics",
						%backtrace,
						"thread panicked: {panic_message}",
					);
				} else {
					error!(
						target: "cs2kz_api::panics",
						"thread panicked: {panic_message}",
					);
				}
			},
			(Some(location), None, Environment::Development) => {
				error!(
					target: "cs2kz_api::panics",
					%location,
					backtrace = %Backtrace::force_capture(),
					"thread panicked",
				);
			},
			(Some(location), None, Environment::Testing | Environment::Production) => {
				let backtrace = Backtrace::capture();

				if backtrace.status() == BacktraceStatus::Captured {
					error!(
						target: "cs2kz_api::panics",
						%location,
						%backtrace,
						"thread panicked",
					);
				} else {
					error!(
						target: "cs2kz_api::panics",
						%location,
						"thread panicked",
					);
				}
			},
			(Some(location), Some(panic_message), Environment::Development) => {
				error!(
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
					error!(
						target: "cs2kz_api::panics",
						%location,
						%backtrace,
						"thread panicked: {panic_message}",
					);
				} else {
					error!(
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
