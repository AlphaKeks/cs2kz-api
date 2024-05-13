//! Custom query parameter types.

use derive_more::{Debug, Display, From, Into};
use serde::{de, Deserialize, Deserializer};
use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::{IntoParams, PartialSchema, ToSchema};

/// Limit the number of returned results.
///
/// Used for pagination.
#[derive(
	Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into, From, sqlx::Type,
)]
#[sqlx(transparent)]
pub struct Limit<const MAX: u64 = 200, const DEFAULT: u64 = 50>(pub u64);

impl<const MAX: u64, const DEFAULT: u64> Default for Limit<MAX, DEFAULT> {
	fn default() -> Self {
		Self(DEFAULT)
	}
}

impl<const MAX: u64, const DEFAULT: u64> From<Limit<MAX, DEFAULT>> for usize {
	#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
	fn from(value: Limit<MAX, DEFAULT>) -> Self {
		value.0 as usize
	}
}

impl<'de, const MAX: u64, const DEFAULT: u64> Deserialize<'de> for Limit<MAX, DEFAULT> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		match Option::<u64>::deserialize(deserializer).map(Option::unwrap_or_default)? {
			value if value <= MAX => Ok(Self(value)),
			value => Err(de::Error::custom(format!(
				"invalid limit `{value}`; cannot exceed `{MAX}`"
			))),
		}
	}
}

impl<const MAX: u64, const DEFAULT: u64> PartialSchema for Limit<MAX, DEFAULT> {
	#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
	fn schema() -> RefOr<Schema> {
		ObjectBuilder::new()
			.description(Some("limits the amount of returned results"))
			.schema_type(SchemaType::Number)
			.minimum(Some(0.0))
			.maximum(Some(MAX as f64))
			.build()
			.into()
	}
}

/// Used for pagination.
#[derive(
	Debug,
	Display,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Into,
	From,
	sqlx::Type,
)]
#[sqlx(transparent)]
pub struct Offset(pub i64);

impl<'de> Deserialize<'de> for Offset {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Option::<i64>::deserialize(deserializer)
			.map(Option::unwrap_or_default)
			.map(Self)
	}
}

impl PartialSchema for Offset {
	fn schema() -> RefOr<Schema> {
		ObjectBuilder::new()
			.description(Some("limits the amount of returned results"))
			.schema_type(SchemaType::Number)
			.minimum(Some(f64::MIN))
			.maximum(Some(f64::MAX))
			.default(Some(0.into()))
			.build()
			.into()
	}
}

/// A generic sorting order for results.
#[derive(
	Debug,
	Display,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deserialize,
	ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum SortingOrder {
	/// Sort from lowest to highest.
	#[default]
	Ascending,

	/// Sort from highest to lowest.
	Descending,
}

impl SortingOrder {
	/// Returns part of a SQL query that can be used in an `ORDER BY` clause.
	pub const fn sql(&self) -> &'static str {
		match *self {
			SortingOrder::Ascending => " ASC ",
			SortingOrder::Descending => " DESC ",
		}
	}
}

/// Helper macro for implementing [`PartialSchema`].
macro_rules! schema {
	($ty:ty as $schema_type:ident, $desc:literal) => {
		impl PartialSchema for $ty {
			fn schema() -> RefOr<Schema> {
				ObjectBuilder::new()
					.schema_type(SchemaType::$schema_type)
					.description(Some($desc))
					.build()
					.into()
			}
		}
	};
}

/// Helper macro for implementing [`IntoParams`].
macro_rules! into_params {
	($ty:ty as $name:literal) => {
		impl IntoParams for $ty {
			fn into_params(
				parameter_in_provider: impl Fn() -> Option<ParameterIn>,
			) -> Vec<Parameter> {
				vec![
					ParameterBuilder::new()
						.name($name)
						.schema(Some(Self::schema()))
						.parameter_in(parameter_in_provider().unwrap_or_default())
						.build(),
				]
			}
		}
	};
}

/// Shim for implementing [`IntoParams`] for a [`cs2kz::SteamID`] path parameter.
pub struct SteamID;

schema!(SteamID as String, "a player's SteamID");
into_params!(SteamID as "steam_id");

/// Shim for implementing [`IntoParams`] for a [`cs2kz::PlayerIdentifier`] path parameter.
pub struct PlayerIdentifier;

schema!(PlayerIdentifier as String, "a player's name or SteamID");
into_params!(PlayerIdentifier as "player");

/// Shim for implementing [`IntoParams`] for a [`MapID`] path parameter.
pub struct MapID;

schema!(MapID as Number, "map's ID");
into_params!(MapID as "map_id");

/// Shim for implementing [`IntoParams`] for a [`cs2kz::MapIdentifier`] path parameter.
pub struct MapIdentifier;

schema!(MapIdentifier as String, "a map's name or ID");
into_params!(MapIdentifier as "map");

/// Shim for implementing [`IntoParams`] for a [`ServerID`] path parameter.
pub struct ServerID;

schema!(ServerID as Number, "server's ID");
into_params!(ServerID as "server_id");

/// Shim for implementing [`IntoParams`] for a [`cs2kz::ServerIdentifier`] path parameter.
pub struct ServerIdentifier;

schema!(ServerIdentifier as String, "a server's name or ID");
into_params!(ServerIdentifier as "server");
