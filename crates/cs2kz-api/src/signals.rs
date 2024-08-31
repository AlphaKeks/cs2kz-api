pub async fn sigint()
{
	match tokio::signal::ctrl_c().await {
		Ok(()) => warn!("received SIGINT"),
		Err(error) => error!(%error, "failed to receive SIGINT"),
	}
}
