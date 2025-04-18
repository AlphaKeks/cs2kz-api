use {
	crate::players::PlayerId,
	serde::{Deserialize, Serialize},
	std::str::FromStr,
	steam_id::{ParseSteamIdError, SteamId},
	utoipa::ToSchema,
};

#[derive(
	Debug,
	Display,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	From,
	Into,
	Serialize,
	Deserialize,
	ToSchema,
)]
#[debug("UserId({})", _0.as_u64())]
#[display("{}", _0.as_u64())]
#[serde(transparent)]
#[schema(value_type = str, format = UInt64, example = "76561198282622073")]
pub struct UserId(#[serde(serialize_with = "SteamId::serialize_u64_stringified")] SteamId);

#[derive(Debug, Display, From, Error)]
pub struct ParseUserIdError(ParseSteamIdError);

impl AsRef<SteamId> for UserId
{
	fn as_ref(&self) -> &SteamId
	{
		&self.0
	}
}

impl PartialEq<PlayerId> for UserId
{
	fn eq(&self, other: &PlayerId) -> bool
	{
		self.as_ref() == other.as_ref()
	}
}

impl FromStr for UserId
{
	type Err = ParseUserIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<SteamId>().map(Self).map_err(ParseUserIdError)
	}
}

impl_rand!(UserId => |rng| UserId(rng.random::<SteamId>()));

impl_sqlx!(UserId => {
	Type as u64;
	Encode<'q> as u64 = |user_id| <UserId as AsRef<SteamId>>::as_ref(&user_id).as_u64();
	Decode<'r> as u64 = |value| SteamId::from_u64(value).map(UserId);
});
