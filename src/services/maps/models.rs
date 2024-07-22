//! Request / Response types for this service.

use std::collections::HashSet;
use std::iter;

use chrono::{DateTime, Utc};
use cs2kz::{GlobalStatus, Mode, RankedStatus, SteamID, Tier};
use serde::{Deserialize, Deserializer, Serialize};
use tap::{Conv, Tap};

use crate::services::players::PlayerInfo;
use crate::services::steam::WorkshopID;
use crate::util::num::ClampedU64;
use crate::util::{self, MapIdentifier};

util::make_id! {
	/// A unique identifier for a KZ map.
	MapID as u16
}

util::make_id! {
	/// A unique identifier for a KZ map course.
	CourseID as u16
}

util::make_id! {
	/// A unique identifier for a KZ map course filter.
	FilterID as u16
}

/// Request payload for fetching a map.
#[derive(Debug)]
pub struct FetchMapRequest
{
	pub ident: MapIdentifier,
}

/// Response payload for fetching a map.
#[derive(Debug, Serialize)]
pub struct FetchMapResponse
{
	pub id: MapID,

	/// The map's name.
	pub name: String,

	/// Description of the map.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// The map's global status.
	pub global_status: GlobalStatus,

	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// CRC32 checksum of the map's `.vpk` file.
	pub checksum: u32,

	/// Players who contributed to the creation of this map.
	pub mappers: Vec<PlayerInfo>,

	/// The map's courses.
	pub courses: Vec<Course>,

	/// When this map was approved.
	pub created_on: DateTime<Utc>,
}

// We can't derive this because of how we use `Vec` here. We aren't _actually_
// decoding arrays here, but just one element and then put that in a `Vec`.
// MySQL doesn't support arrays anyway, so we have to tell sqlx how to decode
// this type manually.
impl<'r, R> sqlx::FromRow<'r, R> for FetchMapResponse
where
	R: sqlx::Row,
	for<'a> &'a str: sqlx::ColumnIndex<R>,
	MapID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	String: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	GlobalStatus: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	WorkshopID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	u32: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	SteamID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	Course: sqlx::FromRow<'r, R>,
	DateTime<Utc>: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
{
	fn from_row(row: &'r R) -> sqlx::Result<Self>
	{
		let id = row.try_get("id")?;
		let name = row.try_get("name")?;
		let description = row.try_get("description")?;
		let global_status = row.try_get("global_status")?;
		let workshop_id = row.try_get("workshop_id")?;
		let checksum = row.try_get("checksum")?;
		let mappers = vec![PlayerInfo {
			name: row.try_get("mapper_name")?,
			steam_id: row.try_get("mapper_id")?,
		}];
		let courses = vec![Course::from_row(row)?];
		let created_on = row.try_get("created_on")?;

		Ok(Self {
			id,
			name,
			description,
			global_status,
			workshop_id,
			checksum,
			mappers,
			courses,
			created_on,
		})
	}
}

/// A KZ map course.
#[derive(Debug, Serialize)]
pub struct Course
{
	/// The course's ID.
	pub id: CourseID,

	/// The course's name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// Description of the course.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// Players who contributed to the creation of this course.
	pub mappers: Vec<PlayerInfo>,

	/// The course's filters.
	pub filters: Vec<Filter>,
}

// We can't derive thish because of how we use `Vec` here.
// See `FetchMapResponse`'s impl for more details.
impl<'r, R> sqlx::FromRow<'r, R> for Course
where
	R: sqlx::Row,
	for<'a> &'a str: sqlx::ColumnIndex<R>,
	CourseID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	String: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	SteamID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	Filter: sqlx::FromRow<'r, R>,
{
	fn from_row(row: &'r R) -> sqlx::Result<Self>
	{
		let id = row.try_get("course_id")?;
		let name = row.try_get("course_name")?;
		let description = row.try_get("course_description")?;
		let mappers = vec![PlayerInfo {
			name: row.try_get("course_mapper_name")?,
			steam_id: row.try_get("course_mapper_id")?,
		}];
		let filters = vec![Filter::from_row(row)?];

		Ok(Self { id, name, description, mappers, filters })
	}
}

/// A KZ map course filter.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Filter
{
	/// The filter's ID.
	#[sqlx(rename = "filter_id")]
	pub id: FilterID,

	/// The mode associated with this filter.
	#[sqlx(rename = "filter_mode")]
	pub mode: Mode,

	/// Whether this filter is for teleport runs.
	#[sqlx(rename = "filter_teleports")]
	pub teleports: bool,

	/// The filter's tier.
	#[sqlx(rename = "filter_tier")]
	pub tier: Tier,

	/// The filter's ranked status.
	#[sqlx(rename = "filter_ranked_status")]
	pub ranked_status: RankedStatus,

	/// Any additional notes.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[sqlx(rename = "filter_notes")]
	pub notes: Option<String>,
}

/// Request payload for fetching maps.
#[derive(Debug, Default, Deserialize)]
pub struct FetchMapsRequest
{
	/// Filter by name.
	pub name: Option<String>,

	/// Filter by workshop ID.
	pub workshop_id: Option<WorkshopID>,

	/// Filter by global status.
	pub global_status: Option<GlobalStatus>,

	/// Only include maps approved after this date.
	pub created_after: Option<DateTime<Utc>>,

	/// Only include maps approved before this date.
	pub created_before: Option<DateTime<Utc>>,

	/// Maximum number of results to return.
	#[serde(default)]
	pub limit: ClampedU64<{ u64::MAX }>,

	/// Pagination offset.
	#[serde(default)]
	pub offset: ClampedU64,
}

/// Response payload for fetching maps.
#[derive(Debug, Serialize)]
pub struct FetchMapsResponse
{
	/// The maps.
	pub maps: Vec<FetchMapResponse>,

	/// How many maps **could have been** fetched, if there was no limit.
	pub total: u64,
}

/// Request payload for submitting a new map.
#[derive(Debug, Deserialize)]
pub struct SubmitMapRequest
{
	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// Description of the map.
	#[serde(default, deserialize_with = "crate::util::serde::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// The map's global status.
	pub global_status: GlobalStatus,

	/// List of SteamIDs of the players who contributed to the creation of this
	/// map.
	#[serde(deserialize_with = "crate::util::serde::deserialize_non_empty")]
	pub mappers: Vec<SteamID>,

	/// The map's courses.
	#[serde(deserialize_with = "SubmitMapRequest::deserialize_courses")]
	pub courses: Vec<NewCourse>,
}

impl SubmitMapRequest
{
	/// Deserializes [`SubmitMapRequest::courses`] and performs validations.
	///
	/// Currently this only checks for duplicate course names.
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<Vec<NewCourse>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let courses = crate::util::serde::deserialize_non_empty::<Vec<NewCourse>, _>(deserializer)?;
		let mut names = HashSet::new();

		if let Some(name) = courses
			.iter()
			.filter_map(|c| c.name.as_deref())
			.find(|&name| !names.insert(name))
		{
			return Err(serde::de::Error::custom(format!(
				"cannot submit duplicate course `{name}`",
			)));
		}

		Ok(courses)
	}
}

/// Request payload for a course when submitting a new map.
#[derive(Debug, Deserialize)]
pub struct NewCourse
{
	/// The course's name.
	#[serde(default, deserialize_with = "crate::util::serde::deserialize_empty_as_none")]
	pub name: Option<String>,

	/// Description of the course.
	#[serde(default, deserialize_with = "crate::util::serde::deserialize_empty_as_none")]
	pub description: Option<String>,

	/// List of SteamIDs of the players who contributed to the creation of this
	/// course.
	#[serde(deserialize_with = "crate::util::serde::deserialize_non_empty")]
	pub mappers: Vec<SteamID>,

	/// The course's filters.
	#[serde(deserialize_with = "NewCourse::deserialize_filters")]
	pub filters: [NewFilter; 4],
}

impl NewCourse
{
	/// Deserializes [`NewCourse::filters`] and performs validations.
	///
	/// Currently this makes sure that:
	/// - the 4 filters are actually the 4 possible permutations
	/// - no T9+ filter is marked as "ranked"
	fn deserialize_filters<'de, D>(deserializer: D) -> Result<[NewFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		/// All the permutations of (mode, runtype) that we expect in a filter.
		const ALL_FILTERS: [(Mode, bool); 4] = [
			(Mode::Vanilla, false),
			(Mode::Vanilla, true),
			(Mode::Classic, false),
			(Mode::Classic, true),
		];

		let filters = <[NewFilter; 4]>::deserialize(deserializer)?.tap_mut(|filters| {
			filters.sort_unstable_by_key(|filter| (filter.mode, filter.teleports));
		});

		for (actual, expected) in iter::zip(&filters, ALL_FILTERS) {
			if (actual.mode, actual.teleports) != expected {
				return Err(serde::de::Error::custom(format!(
					"filter for {} {} is missing",
					expected.0.as_str_short(),
					if expected.1 { "Standard" } else { "Pro" },
				)));
			}

			if actual.tier > Tier::Death && actual.ranked_status.is_ranked() {
				return Err(serde::de::Error::custom(format!(
					"tier {} is too high for a ranked filter",
					actual.tier.conv::<u8>(),
				)));
			}
		}

		Ok(filters)
	}
}

/// Request payload for a course filter when submitting a new map.
#[derive(Debug, Deserialize)]
pub struct NewFilter
{
	/// The mode associated with this filter.
	pub mode: Mode,

	/// Whether this filter is for teleport runs.
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Any additional notes.
	#[serde(default, deserialize_with = "crate::util::serde::deserialize_empty_as_none")]
	pub notes: Option<String>,
}

/// Response payload for submitting a new map.
#[derive(Debug, Serialize)]
pub struct SubmitMapResponse
{
	/// The map's ID.
	pub map_id: MapID,

	/// IDs related to the created courses.
	pub courses: Vec<CreatedCourse>,
}

/// Response payload for created courses when submitting a new map.
#[derive(Debug, Serialize)]
pub struct CreatedCourse
{
	/// The course's ID.
	pub id: CourseID,

	/// The IDS of the course's filters.
	pub filter_ids: [FilterID; 4],
}
