use axum::extract::ws;

#[derive(Debug, Error)]
pub enum DecodeMessageError
{
	#[error("connection closed unexpectedly")]
	ConnectionClosed
	{
		frame: Option<ws::CloseFrame<'static>>,
	},

	#[error("message did not contain a JSON payload")]
	NotJSON,

	#[error("invalid JSON: {source}")]
	InvalidJSON
	{
		/// Message ID, if we managed to decode one.
		id: Option<u64>,

		/// The underlying JSON error.
		source: serde_json::Error,
	},
}

#[derive(Debug, Error)]
#[error("failed to encode message; this is a bug!")]
pub struct EncodeMessageError(#[from] serde_json::Error);
