use {
	crate::steam,
	serde::{Deserialize, Serialize},
	steam_id::SteamId,
	url::Url,
};

const URL: &str = "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

#[derive(Debug, Serialize, Deserialize)]
pub struct User
{
	pub id: SteamId,
	pub name: String,

	#[debug("{:?}", profile_url.as_str())]
	pub profile_url: Url,

	#[debug("{:?}", avatar_url.as_str())]
	pub avatar_url: Url,
}

#[instrument(skip(api_client), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
	api_client: &steam::api::Client,
	user_id: SteamId,
) -> steam::api::Result<Option<User>>
{
	#[derive(serde::Serialize)]
	struct Query<'a>
	{
		#[serde(rename = "key")]
		api_key: &'a str,

		#[serde(rename = "steamids", serialize_with = "SteamId::serialize_u64")]
		user_id: SteamId,
	}

	let request = api_client
		.as_ref()
		.get(URL)
		.query(&Query { api_key: api_client.api_key(), user_id });

	let Response { mut players } = steam::api::send_request(request).await?;

	let player = if players.is_empty() {
		return Ok(None);
	} else {
		players.remove(0)
	};

	Ok(Some(User {
		id: player.steamid,
		name: player.personaname,
		profile_url: player.profileurl,
		avatar_url: player.avatarmedium,
	}))
}

#[derive(Debug, serde::Deserialize)]
struct Response
{
	players: Vec<PlayerObject>,
}

#[derive(Debug, serde::Deserialize)]
struct PlayerObject
{
	steamid: SteamId,
	personaname: String,

	#[debug("{:?}", profileurl.as_str())]
	profileurl: Url,

	#[debug("{:?}", avatarmedium.as_str())]
	avatarmedium: Url,
}
