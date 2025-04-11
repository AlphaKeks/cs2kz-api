use {
	crate::time::Timestamp,
	axum::{
		extract::OptionalFromRequestParts,
		response::{IntoResponse, Response},
	},
	axum_extra::extract::CookieJar,
	http::request,
	serde::{Deserialize, Serialize},
	std::str::FromStr,
	ulid::Ulid,
	zerocopy::IntoBytes,
};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Into, Serialize, Deserialize)]
#[debug("SessionId(\"{_0}\")")]
#[serde(transparent)]
pub struct SessionId(Ulid);

#[derive(Debug, Display, From, Error)]
#[display("failed to parse session ID")]
pub struct ParseSessionIdError(ulid::DecodeError);

impl SessionId
{
	pub const COOKIE_NAME: &str = "kz-auth";

	#[expect(clippy::new_without_default)]
	pub fn new() -> Self
	{
		Self(Ulid::new())
	}

	/// Returns the raw bytes that the session ID consists of.
	pub fn as_bytes(&self) -> &[u8]
	{
		self.0.0.as_bytes()
	}

	/// Returns a timestamp for when this ID was generated.
	pub fn created_at(&self) -> Timestamp
	{
		self.0.datetime().into()
	}
}

impl FromStr for SessionId
{
	type Err = ParseSessionIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<Ulid>().map(Self).map_err(ParseSessionIdError)
	}
}

impl_sqlx!(SessionId => {
	Type as [u8];
	Encode<'q, 'a> as &'a [u8] = |session_id| session_id.as_bytes();
	Decode<'r> as &'r [u8] = |bytes| {
		<[u8; 16]>::try_from(bytes)
			.map(|mut bytes| {
				bytes.reverse();
				Self(Ulid::from_bytes(bytes))
			})
	};
});

#[derive(Debug, Display, Error, From)]
pub struct SessionIdRejection(ParseSessionIdError);

impl IntoResponse for SessionIdRejection
{
	fn into_response(self) -> Response
	{
		http::StatusCode::UNAUTHORIZED.into_response()
	}
}

impl<S> OptionalFromRequestParts<S> for SessionId
where
	S: Send + Sync,
{
	type Rejection = SessionIdRejection;

	#[instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Option<Self>, Self::Rejection>
	{
		if let Some(&maybe_session_id) = parts.extensions.get::<Option<Self>>() {
			return Ok(maybe_session_id);
		}

		let cookies = CookieJar::from_headers(&parts.headers);
		let mut cache_session_id = |maybe_session_id: Option<SessionId>| {
			parts.extensions.insert(maybe_session_id);
		};

		match cookies.get(Self::COOKIE_NAME) {
			None => {
				cache_session_id(None);
				Ok(None)
			},
			Some(cookie) => {
				let session_id = cookie.value().parse()?;
				cache_session_id(Some(session_id));
				Ok(Some(session_id))
			},
		}
	}
}
