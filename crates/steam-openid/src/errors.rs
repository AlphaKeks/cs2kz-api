use thiserror::Error;
use tower_service::Service;

/// Error returned by [`LoginForm::redirect_url()`].
///
/// [`LoginForm::redirect_url()`]: crate::LoginForm::redirect_url()
#[derive(Debug, Error)]
#[error("failed to encode userdata as part of query params")]
pub struct CreateRedirectUrlError(#[from] serde_urlencoded::ser::Error);

/// Error returned by [`CallbackPayload::verify()`].
///
/// [`CallbackPayload::verify()`]: crate::CallbackPayload::verify()
#[derive(Debug, Error)]
pub enum VerifyCallbackPayloadError<S, R>
where
	S: Service<R>,
	S::Error: std::error::Error + 'static,
{
	/// The HTTP client returned an error when called.
	#[error("failed to make http request")]
	HttpClient(#[source] S::Error),

	/// The response body could not be decoded as UTF-8.
	#[error("failed to parse response body as utf-8")]
	ResponseBodyNotUtf8(#[source] std::str::Utf8Error),

	/// The response body did not confirm that the request was valid.
	#[error("payload is invalid")]
	InvalidPayload,
}
