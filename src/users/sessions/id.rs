use {
	axum::{
		extract::OptionalFromRequestParts,
		response::{IntoResponse, Response},
	},
	axum_extra::extract::CookieJar,
	http::request,
	serde::{Deserialize, Serialize},
	std::{error::Error, str::FromStr},
	uuid::Uuid,
};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, From, Into, Serialize, Deserialize)]
#[debug("SessionId({})", _0.as_hyphenated())]
#[display("{}", _0.as_hyphenated())]
#[serde(transparent)]
pub struct SessionId(Uuid);

#[derive(Debug, Display, From, Error)]
#[display("failed to parse session ID")]
pub struct ParseSessionIdError(uuid::Error);

impl SessionId
{
	pub const COOKIE_NAME: &str = "kz-auth";

	#[expect(clippy::new_without_default)]
	pub fn new() -> Self
	{
		Self(Uuid::new_v4())
	}
}

impl FromStr for SessionId
{
	type Err = ParseSessionIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<Uuid>().map(Self).map_err(ParseSessionIdError)
	}
}

impl<DB> sqlx::Type<DB> for SessionId
where
	DB: sqlx::Database,
	[u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<[u8]>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<[u8]>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for SessionId
where
	DB: sqlx::Database,
	for<'a> &'a [u8]: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		(&self.0.as_bytes()[..]).encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		(&self.0.as_bytes()[..]).produces()
	}

	fn size_hint(&self) -> usize
	{
		(&self.0.as_bytes()[..]).size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for SessionId
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		Ok(Self(Uuid::from_bytes(<&'r [u8]>::decode(value)?.try_into()?)))
	}
}

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
