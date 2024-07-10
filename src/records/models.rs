//! Types for modeling KZ records.

use chrono::{DateTime, Utc};
use cs2kz::{Mode, SteamID, Styles};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::{IntoParams, ToSchema};

use crate::kz::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier};
use crate::make_id;
use crate::maps::{CourseID, CourseInfo, MapInfo};
use crate::openapi::parameters::{Limit, Offset, SortingOrder};
use crate::players::Player;
use crate::servers::ServerInfo;
use crate::time::Seconds;

make_id!(RecordID as u64);

/// A KZ record.
#[derive(Debug, Serialize, ToSchema)]
pub struct Record
{
	/// The record's ID.
	pub id: RecordID,

	/// The mode the record was performed in.
	pub mode: Mode,

	/// The styles that were used.
	#[schema(value_type = Vec<String>)]
	pub styles: Styles,

	/// The amount of teleports used.
	pub teleports: u16,

	/// The time in seconds.
	pub time: Seconds,

	/// The player who performed the record.
	pub player: Player,

	/// The map the record was performed on.
	pub map: MapInfo,

	/// The course the record was performed on.
	pub course: CourseInfo,

	/// The server the record was performed on.
	pub server: ServerInfo,

	/// Bhop statistics.
	pub bhop_stats: BhopStats,

	/// When this record was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Record
{
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self>
	{
		Ok(Self {
			id: row.try_get("id")?,
			mode: row.try_get("mode")?,
			styles: row.try_get("styles")?,
			teleports: row.try_get("teleports")?,
			time: row.try_get("time")?,
			player: Player::from_row(row)?,
			map: MapInfo::from_row(row)?,
			course: CourseInfo::from_row(row)?,
			server: ServerInfo::from_row(row)?,
			bhop_stats: BhopStats::from_row(row)?,
			created_on: row.try_get("created_on")?,
		})
	}
}

/// Query parameters for fetching records.
#[derive(Debug, Deserialize, IntoParams)]
pub struct FetchRecordsRequest
{
	/// Filter by mode.
	pub mode: Option<Mode>,

	/// Filter by styles.
	#[serde(default)]
	pub styles: Styles,

	/// Filter by whether teleports were used.
	pub teleports: Option<bool>,

	/// Filter by player.
	pub player: Option<PlayerIdentifier>,

	/// Filter by map.
	pub map: Option<MapIdentifier>,

	/// Filter by course.
	pub course: Option<CourseIdentifier>,

	/// Filter by server.
	pub server: Option<ServerIdentifier>,

	/// Only include records submitted after this date.
	pub created_after: Option<DateTime<Utc>>,

	/// Only include records submitted before this date.
	pub created_before: Option<DateTime<Utc>>,

	/// Which field to sort the results by.
	#[serde(default)]
	pub sort_by: SortRecordsBy,

	/// Which order to sort the results in.
	#[serde(default)]
	pub sort_order: SortingOrder,

	/// Maximum number of results to return.
	#[serde(default)]
	pub limit: Limit,

	/// Pagination offset.
	#[serde(default)]
	pub offset: Offset,
}

/// Fields to sort records by.
#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortRecordsBy
{
	/// Sort by time.
	Time,

	/// Sort by date.
	#[default]
	Date,
}

/// Bhop statistics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BhopStats
{
	/// The amount of bhops.
	pub bhops: u16,

	/// The amount of perfect bhops.
	pub perfs: u16,
}

impl BhopStats
{
	/// Deserializes [`BhopStats`] and checks that `perfs <= bhops`.
	pub fn deserialize_checked<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let bhop_stats = Self::deserialize(deserializer)?;

		if bhop_stats.perfs > bhop_stats.bhops {
			return Err(serde::de::Error::custom("bhop stats can't have more perfs than bhops"));
		}

		Ok(bhop_stats)
	}
}

/// Request payload for creating a new record.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewRecord
{
	/// The SteamID of the player who performed the record.
	pub player_id: SteamID,

	/// The mode the record was performed in.
	pub mode: Mode,

	/// The styles that were used.
	#[serde(default)]
	#[schema(value_type = Vec<String>)]
	pub styles: Styles,

	/// ID of the course the record was performed on.
	pub course_id: CourseID,

	/// The amount of teleports used.
	pub teleports: u16,

	/// The time in seconds.
	pub time: Seconds,

	/// Bhop statistics.
	#[serde(deserialize_with = "BhopStats::deserialize_checked")]
	pub bhop_stats: BhopStats,
}

/// Response body for creating a new record.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedRecord
{
	/// The record's ID.
	pub record_id: RecordID,
}
