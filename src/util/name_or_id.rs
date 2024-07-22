//! A generic abstraction for "identifier"-like types.
//!
//! Many things can be identified either by their name, or some sort of ID.
//! This module exposes various type aliases over [`NameOrID`] for those things.
//! Because they're all so similar, they can share this base type and be
//! distinguished only by the `ID` type parameter.

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::MySql;

use crate::services::maps::{CourseID, MapID};
use crate::services::servers::ServerID;

/// A player name or SteamID.
pub type PlayerIdentifier = NameOrID<SteamID>;

/// A server name or ID.
pub type ServerIdentifier = NameOrID<ServerID>;

/// A map name or ID.
pub type MapIdentifier = NameOrID<MapID>;

/// A course name or ID.
pub type CourseIdentifier = NameOrID<CourseID>;

/// A generic "name or ID".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(clippy::missing_docs_in_private_items)]
pub enum NameOrID<ID>
{
	Name(String),
	ID(ID),
}

impl PlayerIdentifier
{
	/// Returns the SteamID stored in `self` if it exists, or attempts to fetch
	/// it from the database using the name.
	pub async fn resolve_id<'c>(
		&self,
		database: impl sqlx::Executor<'c, Database = MySql>,
	) -> sqlx::Result<Option<SteamID>>
	{
		match self {
			Self::ID(steam_id) => Ok(Some(*steam_id)),
			Self::Name(name) => {
				sqlx::query_scalar! {
					r"
					SELECT
					  id `id: SteamID`
					FROM
					  Players
					WHERE
					  name LIKE ?
					",
					format!("%{name}%"),
				}
				.fetch_optional(database)
				.await
			}
		}
	}
}

impl ServerIdentifier
{
	/// Returns the ID stored in `self` if it exists, or attempts to fetch it
	/// from the database using the name.
	pub async fn resolve_id<'c>(
		&self,
		database: impl sqlx::Executor<'c, Database = MySql>,
	) -> sqlx::Result<Option<ServerID>>
	{
		match self {
			Self::ID(server_id) => Ok(Some(*server_id)),
			Self::Name(name) => {
				sqlx::query_scalar! {
					r"
					SELECT
					  id `id: ServerID`
					FROM
					  Servers
					WHERE
					  name LIKE ?
					",
					format!("%{name}%"),
				}
				.fetch_optional(database)
				.await
			}
		}
	}
}

impl MapIdentifier
{
	/// Returns the ID stored in `self` if it exists, or attempts to fetch it
	/// from the database using the name.
	pub async fn resolve_id<'c>(
		&self,
		database: impl sqlx::Executor<'c, Database = MySql>,
	) -> sqlx::Result<Option<MapID>>
	{
		match self {
			Self::ID(map_id) => Ok(Some(*map_id)),
			Self::Name(name) => {
				sqlx::query_scalar! {
					r"
					SELECT
					  id `id: MapID`
					FROM
					  Maps
					WHERE
					  name LIKE ?
					",
					format!("%{name}%"),
				}
				.fetch_optional(database)
				.await
			}
		}
	}
}

impl CourseIdentifier
{
	/// Returns the ID stored in `self` if it exists, or attempts to fetch it
	/// from the database using the name.
	pub async fn resolve_id<'c>(
		&self,
		database: impl sqlx::Executor<'c, Database = MySql>,
	) -> sqlx::Result<Option<CourseID>>
	{
		match self {
			Self::ID(course_id) => Ok(Some(*course_id)),
			Self::Name(name) => {
				sqlx::query_scalar! {
					r"
					SELECT
					  id `id: CourseID`
					FROM
					  Courses
					WHERE
					  name LIKE ?
					",
					format!("%{name}%"),
				}
				.fetch_optional(database)
				.await
			}
		}
	}
}

impl<ID> fmt::Display for NameOrID<ID>
where
	ID: fmt::Display,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		match self {
			NameOrID::Name(name) => fmt::Display::fmt(name, f),
			NameOrID::ID(id) => fmt::Display::fmt(id, f),
		}
	}
}

impl<ID> FromStr for NameOrID<ID>
where
	ID: FromStr,
{
	type Err = Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		Ok(s.parse::<ID>()
			.map_or_else(|_| Self::Name(s.to_owned()), Self::ID))
	}
}

impl<ID> Serialize for NameOrID<ID>
where
	ID: Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			NameOrID::Name(name) => name.serialize(serializer),
			NameOrID::ID(id) => id.serialize(serializer),
		}
	}
}

// This is not derived because serde prioritizes enum variants in the order of
// their definition, and we want to try the `ID` variant first.
impl<'de, ID> Deserialize<'de> for NameOrID<ID>
where
	ID: Deserialize<'de> + FromStr,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		#[allow(clippy::missing_docs_in_private_items)]
		enum Helper<ID>
		{
			ID(ID),
			Name(String),
		}

		Helper::<ID>::deserialize(deserializer).map(|v| match v {
			Helper::ID(id) => Self::ID(id),

			// Path parameters get deserialized as strings, so we have to parse
			// potential integer types ourselves.
			Helper::Name(name) => name
				.parse::<ID>()
				.map_or_else(|_| Self::Name(name), Self::ID),
		})
	}
}
