//! Custom global panic hook to log runtime panics.
//!
//! See [`std::panic::set_hook()`] for more details.

use std::backtrace::Backtrace;
use std::panic;

/// Installs the API's custom global panic hook.
///
/// The previous hook will be invoked after this custom one is done.
pub fn install()
{
	let old_hook = panic::take_hook();

	panic::set_hook(Box::new(move |info| {
		tracing::error_span!(target: "cs2kz_api::runtime", "panic_hook").in_scope(|| {
			let backtrace = Backtrace::force_capture();

			tracing::error! {
				target: "cs2kz_api::audit_log",
				"\n{info}\n---\nbacktrace:\n{backtrace}",
			};
		});

		old_hook(info)
	}));
}
