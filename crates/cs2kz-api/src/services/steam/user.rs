use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

/// A steam user.
#[derive(Debug, Serialize)]
pub struct User
{
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// The user's SteamID in its stringified 64-bit format.
	#[serde(serialize_with = "SteamID::serialize_u64_stringified")]
	pub steam_id64: SteamID,

	/// The user's username.
	pub username: String,

	/// The user's realname.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub realname: Option<String>,

	/// The user's country.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,

	/// URL to the user's Steam profile.
	pub profile_url: Url,

	/// URL to the user's Steam avatar.
	pub avatar_url: Url,
}

impl<'de> Deserialize<'de> for User
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Helper1
		{
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2
		{
			players: [Helper3; 1],
		}

		#[derive(Deserialize)]
		struct Helper3
		{
			steamid: SteamID,
			personaname: String,
			realname: Option<String>,
			loccountrycode: Option<String>,
			profileurl: Url,
			avatar: Url,
		}

		Helper1::deserialize(deserializer)
			.map(|Helper1 { response }| response)
			.map(|Helper2 { players: [player] }| player)
			.map(|player| Self {
				steam_id: player.steamid,
				steam_id64: player.steamid,
				username: player.personaname,
				realname: player.realname,
				country: player.loccountrycode,
				profile_url: player.profileurl,
				avatar_url: player.avatar,
			})
	}
}
