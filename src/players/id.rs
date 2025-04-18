use {
	crate::users::UserId,
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
#[debug("PlayerId({})", _0.as_u64())]
#[serde(transparent)]
#[schema(value_type = str, format = UInt64, example = "STEAM_1:1:161178172")]
pub struct PlayerId(SteamId);

#[derive(Debug, Display, From, Error)]
pub struct ParsePlayerIdError(ParseSteamIdError);

impl AsRef<SteamId> for PlayerId
{
	fn as_ref(&self) -> &SteamId
	{
		&self.0
	}
}

impl PartialEq<UserId> for PlayerId
{
	fn eq(&self, other: &UserId) -> bool
	{
		self.as_ref() == other.as_ref()
	}
}

impl FromStr for PlayerId
{
	type Err = ParsePlayerIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<SteamId>().map(Self).map_err(ParsePlayerIdError)
	}
}

impl_rand!(PlayerId => |rng| PlayerId(rng.random::<SteamId>()));

impl_sqlx!(PlayerId => {
	Type as u64;
	Encode<'q> as u64 = |player_id| <PlayerId as AsRef<SteamId>>::as_ref(&player_id).as_u64();
	Decode<'r> as u64 = |value| SteamId::from_u64(value).map(PlayerId);
});
