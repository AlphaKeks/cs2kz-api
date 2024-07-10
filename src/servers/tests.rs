use std::net::Ipv6Addr;

use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use reqwest::header;
use uuid::Uuid;

use crate::authentication;
use crate::openapi::responses::PaginationResponse;
use crate::plugin::PluginVersionID;
use crate::servers::{
	self,
	AccessKeyRequest,
	AccessKeyResponse,
	CreatedServer,
	NewServer,
	RefreshKey,
	Server,
	ServerID,
	ServerUpdate,
};

#[crate::integration_test]
async fn fetch_server(ctx: &Context)
{
	let response = ctx
		.http_client
		.get(ctx.url("/servers/alpha"))
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let server = response.json::<Server>().await?;

	assert_eq!(server.name, "Alpha's KZ");
	assert_eq!(server.owner.steam_id, 76561198282622073_u64);
}

#[crate::integration_test]
async fn fetch_servers(ctx: &Context)
{
	let response = ctx
		.http_client
		.get(ctx.url("/servers"))
		.query(&[("limit", "7")])
		.send()
		.await?;

	assert_eq!(response.status(), 200);

	let response = response.json::<PaginationResponse<Server>>().await?;

	assert!(response.results.len() <= 7);
}

#[crate::integration_test(fixtures = ["alphakeks-server-role"])]
async fn approve_server(ctx: &Context)
{
	let alphakeks = SteamID::try_from(76561198282622073_u64)?;
	let server = NewServer {
		name: String::from("very cool server"),
		host: servers::Host::Ipv6(Ipv6Addr::UNSPECIFIED),
		port: 69,
		owned_by: alphakeks,
	};

	let response = ctx
		.http_client
		.post(ctx.url("/servers"))
		.json(&server)
		.send()
		.await?;

	assert_eq!(response.status(), 401);

	let session = ctx.auth_session(alphakeks).await?;
	let session_cookie = Cookie::from(session).encoded().to_string();

	let response = ctx
		.http_client
		.post(ctx.url("/servers"))
		.header(header::COOKIE, session_cookie)
		.json(&server)
		.send()
		.await?;

	assert_eq!(response.status(), 201);

	let CreatedServer { server_id, .. } = response.json().await?;

	let url = ctx.url(format_args!("/servers/{server_id}"));
	let server = ctx
		.http_client
		.get(url)
		.send()
		.await?
		.json::<Server>()
		.await?;

	assert_eq!(server.id, server_id);
	assert_eq!(server.name, "very cool server");
	assert_eq!(server.owner.steam_id, alphakeks);
}

#[crate::integration_test]
async fn update_server(ctx: &Context)
{
	let update = ServerUpdate {
		name: Some(String::from("Church of Schnose")),
		host: None,
		port: None,
		owned_by: None,
	};

	let server = ctx
		.http_client
		.get(ctx.url("/servers/1"))
		.send()
		.await?
		.json::<Server>()
		.await?;

	assert_eq!(server.name, "Alpha's KZ");

	let url = ctx.url(format_args!("/servers/{}", server.id));
	let response = ctx
		.http_client
		.patch(url.clone())
		.json(&update)
		.send()
		.await?;

	assert_eq!(response.status(), 401);

	let alphakeks = SteamID::try_from(76561198282622073_u64)?;
	let session = ctx.auth_session(alphakeks).await?;
	let session_cookie = Cookie::from(session).encoded().to_string();

	let response = ctx
		.http_client
		.patch(url)
		.header(header::COOKIE, session_cookie)
		.json(&update)
		.send()
		.await?;

	assert_eq!(response.status(), 204);

	let server = ctx
		.http_client
		.get(ctx.url("/servers/1"))
		.send()
		.await?
		.json::<Server>()
		.await?;

	assert_eq!(server.name, "Church of Schnose");
}

#[crate::integration_test]
async fn generate_token(ctx: &Context)
{
	let server = sqlx::query! {
		r#"
		SELECT
		  s.id `id: ServerID`,
		  s.refresh_key `refresh_key!: uuid::fmt::Hyphenated`,
		  v.id `plugin_version_id: PluginVersionID`,
		  v.semver
		FROM
		  Servers s
		  JOIN PluginVersions v
		WHERE
		  s.id = 1
		LIMIT
		  1
		"#,
	}
	.fetch_one(&ctx.database)
	.await?;

	let refresh_key = AccessKeyRequest {
		refresh_key: server.refresh_key.into(),
		plugin_version: server.semver.parse()?,
	};

	let response = ctx
		.http_client
		.post(ctx.url("/servers/key"))
		.json(&refresh_key)
		.send()
		.await?;

	assert_eq!(response.status(), 201);

	let AccessKeyResponse { access_key } = response.json().await?;
	let server_info = ctx.decode_jwt::<authentication::Server>(&access_key)?;

	assert_eq!(server_info.id(), server.id);
	assert_eq!(server_info.plugin_version_id(), server.plugin_version_id);
}

#[crate::integration_test(fixtures = ["alphakeks-server-role"])]
async fn replace_key(ctx: &Context)
{
	let server = sqlx::query! {
		r#"
		SELECT
		  refresh_key `refresh_key!: uuid::fmt::Hyphenated`
		FROM
		  Servers
		WHERE
		  id = 1
		"#,
	}
	.fetch_one(&ctx.database)
	.await?;

	let response = ctx
		.http_client
		.put(ctx.url("/servers/1/key"))
		.send()
		.await?;

	assert_eq!(response.status(), 401);

	let alphakeks = SteamID::try_from(76561198282622073_u64)?;
	let session = ctx.auth_session(alphakeks).await?;
	let session_cookie = Cookie::from(session).encoded().to_string();

	let response = ctx
		.http_client
		.put(ctx.url("/servers/1/key"))
		.header(header::COOKIE, session_cookie)
		.send()
		.await?;

	assert_eq!(response.status(), 201);

	let RefreshKey { refresh_key } = response.json().await?;

	assert_ne!(refresh_key, Uuid::from(server.refresh_key));

	let server = sqlx::query! {
		r#"
		SELECT
		  refresh_key `refresh_key!: uuid::fmt::Hyphenated`
		FROM
		  Servers
		WHERE
		  id = 1
		"#,
	}
	.fetch_one(&ctx.database)
	.await?;

	assert_eq!(server.refresh_key, refresh_key.hyphenated());
}

#[crate::integration_test(fixtures = ["alphakeks-server-role"])]
async fn delete_key(ctx: &Context)
{
	let server = sqlx::query! {
		r#"
		SELECT
		  refresh_key `refresh_key: uuid::fmt::Hyphenated`
		FROM
		  Servers
		WHERE
		  id = 1
		"#,
	}
	.fetch_one(&ctx.database)
	.await?;

	assert!(server.refresh_key.is_some());

	let response = ctx
		.http_client
		.delete(ctx.url("/servers/1/key"))
		.send()
		.await?;

	assert_eq!(response.status(), 401);

	let alphakeks = SteamID::try_from(76561198282622073_u64)?;
	let session = ctx.auth_session(alphakeks).await?;
	let session_cookie = Cookie::from(session).encoded().to_string();

	let response = ctx
		.http_client
		.delete(ctx.url("/servers/1/key"))
		.header(header::COOKIE, session_cookie)
		.send()
		.await?;

	assert_eq!(response.status(), 204);

	let server = sqlx::query! {
		r#"
		SELECT
		  refresh_key `refresh_key: uuid::fmt::Hyphenated`
		FROM
		  Servers
		WHERE
		  id = 1
		"#,
	}
	.fetch_one(&ctx.database)
	.await?;

	assert!(server.refresh_key.is_none());
}
