use cs2kz::{Mode, RankedStatus, SteamID, Tier};
use serde::{Deserialize, Deserializer, Serialize};

use crate::database;

make_id! {
	/// An ID uniquely identifying a KZ map.
	pub struct MapID(u16);
}

make_id! {
	/// An ID uniquely identifying a KZ map course.
	pub struct CourseID(u16);
}

make_id! {
	/// An ID uniquely identifying a KZ map course filter.
	pub struct FilterID(u16);
}

/// A map ID or name.
#[derive(Debug, Clone)]
pub enum MapIdentifier
{
	/// A map ID.
	ID(MapID),

	/// A name.
	Name(String),
}

impl MapIdentifier
{
	/// Returns the ID contained in `self` or fetches it from the database by looking up
	/// the name.
	pub async fn resolve_id(
		&self,
		conn: impl database::Executor<'_>,
	) -> database::Result<Option<MapID>>
	{
		match *self {
			Self::ID(map_id) => Ok(Some(map_id)),
			Self::Name(ref name) => {
				sqlx::query_scalar! {
					"SELECT id `id: MapID`
					 FROM Maps
					 WHERE name LIKE ?",
					format!("%{name}%"),
				}
				.fetch_optional(conn)
				.await
			}
		}
	}
}

impl<'de> Deserialize<'de> for MapIdentifier
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Debug, Deserialize)]
		#[serde(untagged)]
		pub enum Helper
		{
			ID(MapID),
			Str(String),
		}

		Helper::deserialize(deserializer).map(|v| match v {
			Helper::ID(map_id) => Self::ID(map_id),
			Helper::Str(str) => str
				.parse::<MapID>()
				.map_or_else(|_| Self::Name(str), Self::ID),
		})
	}
}

/// A mapper.
#[derive(Debug, Serialize)]
pub struct Mapper
{
	/// The user's name.
	pub name: String,

	/// The user's SteamID.
	pub steam_id: SteamID,
}

/// A KZ map course.
#[derive(Debug, Serialize)]
pub struct Course
{
	/// The course ID.
	pub id: CourseID,

	/// The course name.
	pub name: String,

	/// The course description.
	pub description: String,

	/// List of players who have contributed to making this course.
	pub mappers: Vec<Mapper>,

	/// The course filters.
	pub filters: [Filter; 4],
}

/// A KZ map course filter.
#[derive(Debug, Serialize)]
pub struct Filter
{
	/// The filter's ID.
	pub id: FilterID,

	/// The filter's mode.
	pub mode: Mode,

	/// Whether this filter applies to runs with or without teleports.
	pub has_teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Notes about this filter.
	pub notes: String,
}
