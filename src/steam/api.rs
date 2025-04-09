use {
	bytes::Bytes,
	http_body_util::BodyExt,
	reqwest::RequestBuilder,
	serde::Deserialize,
	std::{fmt, sync::Arc},
};

pub(super) type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Display, Error, From)]
#[display("Steam API error: {_variant}")]
pub enum ApiError
{
	#[display("failed to make http request")]
	Http(reqwest::Error),

	#[display("failed to buffer response body")]
	#[from(ignore)]
	BufferResponseBody
	{
		#[error(source)]
		error: reqwest::Error,
		response: http::response::Parts,
	},

	#[display("failed to deserialize response from Steam")]
	#[from(ignore)]
	DeserializeResponse
	{
		#[error(source)]
		error: serde_json::Error,
		response: http::Response<Bytes>,
	},
}

#[derive(Debug, Clone)]
pub struct Client
{
	http_client: reqwest::Client,
	api_key: Arc<str>,
}

impl Client
{
	pub fn new(api_key: impl Into<Arc<str>>) -> Self
	{
		Self { http_client: reqwest::Client::default(), api_key: api_key.into() }
	}

	pub(super) fn api_key(&self) -> &str
	{
		&self.api_key
	}
}

impl AsRef<reqwest::Client> for Client
{
	fn as_ref(&self) -> &reqwest::Client
	{
		&self.http_client
	}
}

#[instrument(level = "debug", ret(level = "debug"), err(Debug, level = "debug"))]
pub(super) async fn send_request<T>(request: RequestBuilder) -> Result<T>
where
	T: fmt::Debug + for<'de> Deserialize<'de>,
{
	#[derive(Debug, serde::Deserialize)]
	struct ApiResponse<T>
	{
		response: T,
	}

	let response = request.send().await?;

	if let Err(error) = response.error_for_status_ref() {
		return Err(ApiError::Http(error));
	}

	let (response, body) = http::Response::from(response).into_parts();
	let body = match body.collect().await {
		Ok(collected) => collected.to_bytes(),
		Err(error) => return Err(ApiError::BufferResponseBody { error, response }),
	};

	serde_json::from_slice(&body[..])
		.map(|ApiResponse { response }| response)
		.map_err(|err| ApiError::DeserializeResponse {
			error: err,
			response: http::Response::from_parts(response, body),
		})
}
