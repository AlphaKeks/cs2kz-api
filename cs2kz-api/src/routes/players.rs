// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{responses, Result, State},
	axum::{http::StatusCode, Json},
	cs2kz::SteamID,
	serde::Deserialize,
	utoipa::ToSchema,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePlayer {
	steam_id: SteamID,
	name: String,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Players", context_path = "/api/v0", path = "/players", request_body = CreatePlayer, responses(
	(status = 201, description = "Player has been inserted into the database."),
	(status = 400, response = responses::BadRequest),
	(status = 401, response = responses::Unauthorized),
	(status = 500, response = responses::Database),
))]
pub async fn create(
	state: State,
	Json(CreatePlayer { steam_id, name }): Json<CreatePlayer>,
) -> Result<StatusCode> {
	sqlx::query! {
		r#"
		INSERT INTO
			Players (id, name)
		VALUES
			(?, ?)
		"#,
		steam_id.as_u32(),
		name,
	}
	.execute(state.database())
	.await?;

	Ok(StatusCode::CREATED)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePlayer {
	#[serde(flatten)]
	player: CreatePlayer,
	additional_playtime: u32,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Players", context_path = "/api/v0", path = "/players", request_body = UpdatePlayer, responses(
	(status = 200, description = "Player has been updated successfully."),
	(status = 400, response = responses::BadRequest),
	(status = 401, response = responses::Unauthorized),
	(status = 500, response = responses::Database),
))]
pub async fn update(
	state: State,
	Json(UpdatePlayer { player, additional_playtime }): Json<UpdatePlayer>,
) -> Result<StatusCode> {
	sqlx::query! {
		r#"
		UPDATE
			Players
		SET
			name = ?,
			playtime = playtime + ?
		WHERE
			id = ?
		"#,
		player.name,
		additional_playtime,
		player.steam_id.as_u32(),
	}
	.execute(state.database())
	.await?;

	Ok(StatusCode::OK)
}
