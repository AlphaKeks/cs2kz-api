use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use cs2kz::SteamID;
use serde_json::{json, Value as JsonValue};
use tokio::time::sleep;
use uuid::Uuid;

use crate::game_sessions::TimeSpent;
use crate::openapi::responses::PaginationResponse;
use crate::players::{FullPlayer, NewPlayer, PlayerUpdate, Session};
use crate::records::BhopStats;

#[crate::integration_test]
async fn fetch_player(ctx: &Context)
{
	let response = ctx
		.http_client
		.get(ctx.url("/players/alphakeks"))
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let alphakeks = response.json::<FullPlayer>().await?;

	assert_eq!(alphakeks.name, "AlphaKeks");
	assert_eq!(alphakeks.steam_id, 76561198282622073_u64);
}

#[crate::integration_test]
async fn fetch_players(ctx: &Context)
{
	let response = ctx
		.http_client
		.get(ctx.url("/players"))
		.query(&[("limit", "7")])
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let response = response.json::<PaginationResponse<FullPlayer>>().await?;

	assert!(response.results.len() <= 7);
}

#[crate::integration_test]
async fn register_player(ctx: &Context)
{
	let player = NewPlayer {
		name: String::from("AlphaKeks"),
		steam_id: SteamID::try_from(76561198282622073_u64)?,
		ip_address: Ipv6Addr::LOCALHOST.into(),
	};

	let missing_auth_header = ctx
		.http_client
		.post(ctx.url("/players"))
		.json(&player)
		.send()
		.await?;

	assert_eq!(missing_auth_header.status(), 400);

	let jwt = ctx.auth_server(Duration::from_secs(0))?;

	sleep(Duration::from_secs(1)).await;

	let unauthorized = ctx
		.http_client
		.post(ctx.url("/players"))
		.header("Authorization", format!("Bearer {jwt}"))
		.json(&player)
		.send()
		.await?;

	assert_eq!(unauthorized.status(), 401);

	let jwt = ctx.auth_server(Duration::from_secs(60 * 60))?;

	let already_exists = ctx
		.http_client
		.post(ctx.url("/players"))
		.header("Authorization", format!("Bearer {jwt}"))
		.json(&player)
		.send()
		.await?;

	assert_eq!(already_exists.status(), 409);

	let new_ip = Ipv4Addr::new(69, 69, 69, 69);
	let new_player = NewPlayer {
		name: String::from("very cool person"),
		steam_id: SteamID::MAX,
		ip_address: new_ip.into(),
	};

	let success = ctx
		.http_client
		.post(ctx.url("/players"))
		.header("Authorization", format!("Bearer {jwt}"))
		.json(&new_player)
		.send()
		.await?;

	assert_eq!(success.status(), 201);

	let player = ctx
		.http_client
		.get(ctx.url(format!("/players/{}", new_player.steam_id)))
		.send()
		.await?
		.json::<FullPlayer>()
		.await?;

	assert_eq!(new_player.name, player.name);
	assert!(player.ip_address.and_then(|ip| ip.to_ipv4_mapped()) == Some(new_ip));
}

#[crate::integration_test]
async fn update_player(ctx: &Context)
{
	let response = ctx
		.http_client
		.get(ctx.url("/players/alphakeks"))
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let player = response.json::<FullPlayer>().await?;
	let new_name = player.name.chars().rev().collect::<String>();
	let new_ip = Ipv4Addr::new(69, 69, 69, 69).into();

	let update = PlayerUpdate {
		name: new_name.clone(),
		ip_address: new_ip,
		session: Session {
			time_spent: TimeSpent {
				active: Duration::from_secs(6942).into(),
				spectating: Duration::from_secs(1337).into(),
				afk: Duration::from_secs(0).into(),
			},
			bhop_stats: BhopStats { bhops: 13847, perfs: 6237 },
			course_sessions: BTreeMap::new(),
		},
		preferences: json!({ "funny_test": ctx.test_id }),
	};

	let url = ctx.url(format_args!("/players/{}", player.steam_id));
	let jwt = ctx.auth_server(Duration::from_secs(60 * 60))?;

	let response = ctx
		.http_client
		.patch(url)
		.header("Authorization", format!("Bearer {jwt}"))
		.json(&update)
		.send()
		.await?;

	assert_eq!(response.status(), 204);

	let actual_ip = sqlx::query_scalar! {
		r#"
			SELECT
			  ip_address `ip: IpAddr`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
		player.steam_id,
	}
	.fetch_one(&ctx.database)
	.await?;

	match (new_ip, actual_ip) {
		(IpAddr::V4(new), IpAddr::V4(actual)) => {
			assert_eq!(new, actual);
		}
		(IpAddr::V6(new), IpAddr::V6(actual)) => {
			assert_eq!(new, actual);
		}
		(IpAddr::V4(new), IpAddr::V6(actual)) => {
			assert_eq!(new.to_ipv6_mapped(), actual);
		}
		(IpAddr::V6(new), IpAddr::V4(actual)) => {
			assert_eq!(new, actual.to_ipv6_mapped());
		}
	}

	let url = ctx.url(format_args!("/players/{}", player.steam_id));
	let response = ctx.http_client.get(url).send().await?;

	assert_eq!(response.status(), 200);

	let player = response.json::<FullPlayer>().await?;

	assert_eq!(player.name, new_name);

	let url = ctx.url(format_args!("/players/{}/preferences", player.steam_id));
	let response = ctx.http_client.get(url).send().await?;

	assert_eq!(response.status(), 200);

	let mut preferences = response.json::<JsonValue>().await?;
	let funny_test = preferences
		.get_mut("funny_test")
		.map(JsonValue::take)
		.map(serde_json::from_value::<Uuid>)
		.expect("this cannot fail")?;

	assert_eq!(funny_test, ctx.test_id);
}
