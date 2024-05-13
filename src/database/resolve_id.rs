//! A trait for *maybe* querying the database.
//!
//! Many handlers take "SomethingIdentifier" types as input, where you either get a generic name,
//! or an exact ID. If the ID is not present, it should be fetched from the database. This is a
//! common pattern, and so the [`ResolveID`] trait acts as a convenience extension trait to perform
//! this operation on those types directly.

use std::future::Future;

use cs2kz::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier, SteamID};
use sqlx::MySql;

use crate::maps::{CourseID, MapID};
use crate::servers::ServerID;

/// Helper trait for querying the database for IDs.
pub trait ResolveID {
	/// The ID that should be resolved.
	type ID;

	/// Resolve `Self::ID` by potentially querying the database.
	fn resolve_id<'conn, E>(
		&self,
		executor: E,
	) -> impl Future<Output = sqlx::Result<Option<<Self as ResolveID>::ID>>> + Send
	where
		E: sqlx::Executor<'conn, Database = MySql>;
}

impl ResolveID for PlayerIdentifier {
	type ID = SteamID;

	async fn resolve_id<'conn, E>(
		&self,
		executor: E,
	) -> sqlx::Result<Option<<Self as ResolveID>::ID>>
	where
		E: sqlx::Executor<'conn, Database = MySql>,
	{
		let name = match *self {
			Self::Name(ref name) => name,
			Self::SteamID(steam_id) => return Ok(Some(steam_id)),
		};

		let steam_id = sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: SteamID`
			FROM
			  Players
			WHERE
			  name LIKE ?
			"#,
			format!("%{name}%"),
		}
		.fetch_optional(executor)
		.await?;

		Ok(steam_id)
	}
}

impl ResolveID for MapIdentifier {
	type ID = MapID;

	async fn resolve_id<'conn, E>(
		&self,
		executor: E,
	) -> sqlx::Result<Option<<Self as ResolveID>::ID>>
	where
		E: sqlx::Executor<'conn, Database = MySql>,
	{
		let name = match *self {
			Self::Name(ref name) => name,
			Self::ID(map_id) => return Ok(Some(map_id.into())),
		};

		let map_id = sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: MapID`
			FROM
			  Maps
			WHERE
			  name LIKE ?
			"#,
			format!("%{name}%"),
		}
		.fetch_optional(executor)
		.await?;

		Ok(map_id)
	}
}

impl ResolveID for CourseIdentifier {
	type ID = CourseID;

	async fn resolve_id<'conn, E>(
		&self,
		executor: E,
	) -> sqlx::Result<Option<<Self as ResolveID>::ID>>
	where
		E: sqlx::Executor<'conn, Database = MySql>,
	{
		let name = match *self {
			Self::Name(ref name) => name,
			Self::ID(course_id) => return Ok(Some(course_id.into())),
		};

		let course_id = sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: CourseID`
			FROM
			  Courses
			WHERE
			  name LIKE ?
			"#,
			format!("%{name}%"),
		}
		.fetch_optional(executor)
		.await?;

		Ok(course_id)
	}
}

impl ResolveID for ServerIdentifier {
	type ID = ServerID;

	async fn resolve_id<'conn, E>(
		&self,
		executor: E,
	) -> sqlx::Result<Option<<Self as ResolveID>::ID>>
	where
		E: sqlx::Executor<'conn, Database = MySql>,
	{
		let name = match *self {
			Self::Name(ref name) => name,
			Self::ID(server_id) => return Ok(Some(server_id.into())),
		};

		let server_id = sqlx::query_scalar! {
			r#"
			SELECT
			  id `id: ServerID`
			FROM
			  Servers
			WHERE
			  name LIKE ?
			"#,
			format!("%{name}%"),
		}
		.fetch_optional(executor)
		.await?;

		Ok(server_id)
	}
}
