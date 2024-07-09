//! A generic abstraction for "identifier"-like types.
//!
//! Many things can be identified either by their name, or some sort of ID.
//! This module exposes various type aliases over [`NameOrID`] for those things.
//! Because they're all so similar, they can share this base type and be
//! distinguished only by the `ID` type parameter.

#![allow(private_interfaces)]

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, OneOfBuilder, RefOr, SchemaType};
use utoipa::{IntoParams, PartialSchema, ToSchema};

use crate::maps::{CourseID, MapID};
use crate::servers::ServerID;

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

impl PartialSchema for PlayerIdentifier
{
	fn schema() -> RefOr<Schema>
	{
		Schema::OneOf(
			OneOfBuilder::new()
				.description(Some("a player's SteamID or name"))
				.example(Some("AlphaKeks".into()))
				.item(<SteamID as PartialSchema>::schema())
				.item(Schema::Object(
					ObjectBuilder::new()
						.description(Some("a player's name"))
						.example(Some("AlphaKeks".into()))
						.schema_type(SchemaType::String)
						.build(),
				))
				.build(),
		)
		.into()
	}
}

impl<'s> ToSchema<'s> for PlayerIdentifier
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		("PlayerIdentifier", <Self as PartialSchema>::schema())
	}
}

impl IntoParams for PlayerIdentifier
{
	fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
	{
		vec![
			ParameterBuilder::new()
				.name("player")
				.parameter_in(parameter_in_provider().unwrap_or_default())
				.description(Some("a player's SteamID or name"))
				.schema(Some(<Self as PartialSchema>::schema()))
				.example(Some("alphakeks".into()))
				.build(),
		]
	}
}

/// Implements [`utoipa`] traits for all the different identifier types.
macro_rules! impl_schema {
	(
		$id:ty,
		$name:ident,
		$param_name:literal,
		$desc:literal,
		$name_desc:literal,
		$example:literal $(,)?
	) => {
		impl PartialSchema for NameOrID<$id>
		{
			fn schema() -> RefOr<Schema>
			{
				Schema::OneOf(
					OneOfBuilder::new()
						.description(Some($desc))
						.example(Some($example.into()))
						.item(<$id as ToSchema>::schema().1)
						.item(Schema::Object(
							ObjectBuilder::new()
								.description(Some($name_desc))
								.example(Some($example.into()))
								.schema_type(SchemaType::String)
								.build(),
						))
						.build(),
				)
				.into()
			}
		}

		impl<'s> ToSchema<'s> for NameOrID<$id>
		{
			fn schema() -> (&'s str, RefOr<Schema>)
			{
				(stringify!($name), <Self as PartialSchema>::schema())
			}
		}

		impl IntoParams for NameOrID<$id>
		{
			fn into_params(
				parameter_in_provider: impl Fn() -> Option<ParameterIn>,
			) -> Vec<Parameter>
			{
				vec![
					ParameterBuilder::new()
						.name($param_name)
						.parameter_in(parameter_in_provider().unwrap_or_default())
						.description(Some($desc))
						.schema(Some(<Self as PartialSchema>::schema()))
						.example(Some($example.into()))
						.build(),
				]
			}
		}
	};
}

impl_schema! {
	ServerID,
	ServerIdentifier,
	"server",
	"a server ID or name",
	"a server name",
	"Alpha's KZ",
}

impl_schema! {
	MapID,
	MapIdentifier,
	"map",
	"a map ID or name",
	"a map name",
	"kz_checkmate",
}

impl_schema! {
	CourseID,
	CourseIdentifier,
	"course",
	"a course ID or name",
	"a course name",
	"super cool course name",
}
