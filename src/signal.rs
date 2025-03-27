use std::error::Error;

use tokio::signal::ctrl_c;

/// Listens for a shutdown signal from the OS.
pub(crate) async fn shutdown()
{
	select! {
		ctrl_c_result = ctrl_c() => match ctrl_c_result {
			Ok(()) => tracing::debug!("received SIGINT"),
			Err(err) => {
				tracing::error!(error = &err as &dyn Error, "failed listening for SIGINT");
			},
		},

		() = platform_specific_shutdown() => {},
	}
}

#[cfg(unix)]
async fn platform_specific_shutdown()
{
	use tokio::signal::unix::{SignalKind, signal};

	match signal(SignalKind::terminate()) {
		Ok(mut signal) => match signal.recv().await {
			Some(()) => tracing::debug!("received SIGTERM"),
			None => tracing::warn!("cannot receive more SIGTERM signals"),
		},
		Err(err) => {
			tracing::error!(error = &err as &dyn Error, "failed listening for SIGTERM");
		},
	}
}

#[cfg(not(unix))]
async fn platform_specific_shutdown()
{
	std::future::pending().await
}
