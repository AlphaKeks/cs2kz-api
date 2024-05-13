//! Shared data types for this module.

use std::collections::{btree_map, BTreeMap, HashSet};
use std::iter;

use chrono::{DateTime, Utc};
use cs2kz::{GlobalStatus, Mode, RankedStatus, SteamID, Tier};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::steam::workshop::WorkshopID;

#[cs2kz_api_macros::id]
pub struct MapID(pub u16);

#[cs2kz_api_macros::id]
pub struct CourseID(pub u16);

#[cs2kz_api_macros::id]
pub struct FilterID(pub u16);

/// A KZ map.
#[derive(Debug, Serialize, ToSchema)]
pub struct FullMap {
	/// The map's ID.
	pub id: MapID,

	/// The map's name.
	pub name: String,

	/// A description of the map.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// The current global status of the map.
	pub global_status: GlobalStatus,

	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// The map's file checksum.
	pub checksum: u32,

	/// The map's creators.
	pub mappers: Vec<Player>,

	/// The map's courses.
	pub courses: Vec<Course>,

	/// When this map was approved.
	pub created_on: DateTime<Utc>,
}

impl FullMap {
	/// Merges `other` into `self` by taking `other`'s [`mappers`] and [`courses`].
	///
	/// # Panics
	///
	/// This function will panic if `self` has a different [`id`] than `other`.
	///
	/// [`mappers`]: FullMap::mappers
	/// [`courses`]: FullMap::courses
	/// [`id`]: FullMap::id
	pub fn merge(mut self, other: Self) -> Self {
		assert_eq!(self.id, other.id, "merging two unrelated maps");

		for mapper in other.mappers {
			if !self.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
				self.mappers.push(mapper);
			}
		}

		for course in other.courses {
			let Some(c) = self.courses.iter_mut().find(|c| c.id == course.id) else {
				self.courses.push(course);
				continue;
			};

			for mapper in course.mappers {
				if !c.mappers.iter().any(|m| m.steam_id == mapper.steam_id) {
					c.mappers.push(mapper);
				}
			}

			for filter in course.filters {
				if !c.filters.iter().any(|m| m.id == filter.id) {
					c.filters.push(filter);
				}
			}
		}

		self
	}

	/// Normalizes the results of a SQL query by merging together maps with equal IDs.
	pub fn normalize_sql_results<I>(maps: I, limit: usize) -> Vec<Self>
	where
		I: IntoIterator<Item = Self>,
	{
		maps.into_iter()
			.group_by(|map| map.id)
			.into_iter()
			.filter_map(|(_, group)| group.reduce(Self::merge))
			.take(limit)
			.collect()
	}
}

impl FromRow<'_, MySqlRow> for FullMap {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			name: row.try_get("name")?,
			description: row.try_get("description")?,
			global_status: row.try_get("global_status")?,
			workshop_id: row.try_get("workshop_id")?,
			checksum: row.try_get("checksum")?,
			mappers: vec![Player {
				name: row.try_get("mapper_name")?,
				steam_id: row.try_get("mapper_id")?,
			}],
			courses: vec![Course::from_row(row)?],
			created_on: row.try_get("created_on")?,
		})
	}
}

/// A KZ map course.
#[derive(Debug, Serialize, ToSchema)]
pub struct Course {
	/// The course's ID.
	pub id: CourseID,

	/// The course's name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// A description of the course.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// The course's creators.
	pub mappers: Vec<Player>,

	/// The course's filters.
	pub filters: Vec<CourseFilter>,
}

impl FromRow<'_, MySqlRow> for Course {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			name: row.try_get("name")?,
			description: row
				.try_get::<Option<String>, _>("description")?
				.filter(|s| !s.is_empty()),
			mappers: vec![Player {
				name: row.try_get("course_mapper_name")?,
				steam_id: row.try_get("course_mapper_id")?,
			}],
			filters: vec![CourseFilter::from_row(row)?],
		})
	}
}

/// A KZ map course filter.
///
/// A filter is a combination of mode and whether teleports are allowed.
/// Since there are 2 modes, there are 4 filters per course.
/// Each filter has its own [tier] and [ranked status].
///
/// [tier]: Tier
/// [ranked status]: RankedStatus
#[derive(Debug, Serialize, ToSchema)]
pub struct CourseFilter {
	/// The filter's ID.
	pub id: FilterID,

	/// The filter's mode.
	pub mode: Mode,

	/// Whether teleports are allowed.
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Notes about the filter.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub notes: Option<String>,
}

impl FromRow<'_, MySqlRow> for CourseFilter {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			mode: row.try_get("mode")?,
			teleports: row.try_get("teleports")?,
			tier: row.try_get("tier")?,
			ranked_status: row.try_get("ranked_status")?,
			notes: row
				.try_get::<Option<String>, _>("notes")?
				.filter(|s| !s.is_empty()),
		})
	}
}

/// Request payload for creating new maps.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMap {
	/// A description of the map.
	#[serde(with = "crate::serde::empty_as_none::string")]
	pub description: Option<String>,

	/// The map's initial global status.
	pub global_status: GlobalStatus,

	/// The map's Steam Workshop ID.
	pub workshop_id: WorkshopID,

	/// The SteamIDs of the map's creators.
	#[serde(with = "crate::serde::non_empty::vec")]
	pub mappers: Vec<SteamID>,

	/// The map's courses.
	#[serde(deserialize_with = "NewMap::deserialize_courses")]
	pub courses: Vec<NewCourse>,
}

impl NewMap {
	/// Deserializes submitted courses and makes sure their names are unique.
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<Vec<NewCourse>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let courses: Vec<NewCourse> = crate::serde::non_empty::vec::deserialize(deserializer)?;
		let mut course_names = HashSet::new();

		for name in courses.iter().filter_map(|course| course.name.as_deref()) {
			if !course_names.insert(name) {
				return Err(serde::de::Error::custom(format!(
					"cannot submit duplicate courses (`{name}`)"
				)));
			}
		}

		Ok(courses)
	}
}

/// Request payload for creating new map courses.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewCourse {
	/// The course's name.
	#[serde(with = "crate::serde::empty_as_none::string")]
	pub name: Option<String>,

	/// A description of the course.
	#[serde(with = "crate::serde::empty_as_none::string")]
	pub description: Option<String>,

	/// The SteamIDs of the course's creators.
	#[serde(with = "crate::serde::non_empty::vec")]
	pub mappers: Vec<SteamID>,

	/// The course's filters.
	#[serde(deserialize_with = "NewCourse::deserialize_filters")]
	pub filters: [NewFilter; 4],
}

impl NewCourse {
	/// Deserializes and validates submitted filters.
	fn deserialize_filters<'de, D>(deserializer: D) -> Result<[NewFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut filters = <[NewFilter; 4]>::deserialize(deserializer)?;

		filters.sort_unstable_by_key(|f| (f.mode, f.teleports));

		/// The expected set of filters.
		const EXPECTED: [(Mode, bool); 4] = [
			(Mode::Vanilla, false),
			(Mode::Vanilla, true),
			(Mode::Classic, false),
			(Mode::Classic, true),
		];

		for (filter, expected) in iter::zip(&filters, EXPECTED) {
			if (filter.mode, filter.teleports) != expected {
				return Err(serde::de::Error::custom(format_args!(
					"filter for ({}, {}) is missing",
					filter.mode,
					if filter.teleports { "TP" } else { "Pro" },
				)));
			}

			if filter.tier > Tier::Death && filter.ranked_status.is_ranked() {
				return Err(serde::de::Error::custom(format_args!(
					"tier `{}` is too high for a ranked filter",
					filter.tier,
				)));
			}
		}

		Ok(filters)
	}
}

/// Request payload for creating new map course filters.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewFilter {
	/// The filter's mode.
	pub mode: Mode,

	/// Whether teleports are allowed.
	pub teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Notes about the filter.
	#[serde(with = "crate::serde::empty_as_none::string")]
	pub notes: Option<String>,
}

/// Response body for creating a new map.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedMap {
	/// The map's ID.
	pub map_id: MapID,

	/// The course IDs for the map's courses.
	pub course_ids: Vec<CourseID>,

	/// The course's filter IDs.
	pub filter_ids: BTreeMap<CourseID, [FilterID; 4]>,
}

/// Request payload for updating a map.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MapUpdate {
	/// A new description.
	pub description: Option<String>,

	/// A new global status.
	pub global_status: Option<GlobalStatus>,

	/// A new Workshop ID.
	///
	/// Implies `check_workshop=true`.
	pub workshop_id: Option<WorkshopID>,

	/// Check the Workshop for a new name / checksum.
	#[serde(default)]
	pub check_workshop: bool,

	/// List of SteamIDs to add as mappers.
	#[serde(default, with = "crate::serde::empty_as_none::vec")]
	pub added_mappers: Option<Vec<SteamID>>,

	/// List of SteamIDs to remove as mappers.
	#[serde(default, with = "crate::serde::empty_as_none::vec")]
	pub removed_mappers: Option<Vec<SteamID>>,

	/// Updates to courses of the map.
	#[serde(default, with = "crate::serde::empty_as_none::btree_map")]
	pub course_updates: Option<BTreeMap<CourseID, CourseUpdate>>,
}

impl MapUpdate {
	/// Checks if a map update is empty, i.e., only holds empty containers.
	pub fn is_empty(&self) -> bool {
		let Self {
			description,
			global_status,
			workshop_id,
			check_workshop,
			added_mappers,
			removed_mappers,
			course_updates,
		} = self;

		description.is_none()
			&& global_status.is_none()
			&& workshop_id.is_none()
			&& !check_workshop
			&& added_mappers.is_none()
			&& removed_mappers.is_none()
			&& course_updates.is_none()
	}
}

/// A collection of course filter updates.
pub type FilterUpdates = BTreeMap<(Mode, bool), FilterUpdate>;

/// An update to a map course's metadata.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CourseUpdate {
	/// The course's name.
	pub name: Option<Option<String>>,

	/// A description of the course.
	pub description: Option<Option<String>>,

	/// List of SteamIDs to add as mappers.
	#[serde(default, with = "crate::serde::empty_as_none::vec")]
	pub added_mappers: Option<Vec<SteamID>>,

	/// List of SteamIDs to remove as mappers.
	#[serde(default, with = "crate::serde::empty_as_none::vec")]
	pub removed_mappers: Option<Vec<SteamID>>,

	/// Updates to the course's filters.
	#[serde(default, deserialize_with = "CourseUpdate::deserialize_filter_updates")]
	pub filter_updates: Option<FilterUpdates>,
}

impl CourseUpdate {
	/// Checks if a course update is empty, i.e., only holds empty containers.
	pub const fn is_empty(&self) -> bool {
		let Self {
			name,
			description,
			added_mappers,
			removed_mappers,
			filter_updates,
		} = self;

		name.is_none()
			&& description.is_none()
			&& added_mappers.is_none()
			&& removed_mappers.is_none()
			&& filter_updates.is_none()
	}

	/// Deserializes and validates filter updates.
	fn deserialize_filter_updates<'de, D>(
		deserializer: D,
	) -> Result<Option<FilterUpdates>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let Some(raw_updates) = crate::serde::empty_as_none::vec::deserialize(deserializer)? else {
			return Ok(None);
		};

		let mut updates = BTreeMap::new();

		for RawFilterUpdate {
			mode,
			teleports,
			update,
		} in raw_updates
		{
			if let (Some(tier), Some(ranked_status)) = (update.tier, update.ranked_status) {
				if tier > Tier::Death && ranked_status.is_ranked() {
					return Err(serde::de::Error::custom(format_args!(
						"tier `{tier}` is too high for a ranked filter ({mode}, {runtype})",
						runtype = if teleports { "TP" } else { "Pro" },
					)));
				}
			}

			match updates.entry((mode, teleports)) {
				btree_map::Entry::Vacant(entry) => {
					entry.insert(update);
				}
				btree_map::Entry::Occupied(_) => {
					return Err(serde::de::Error::custom(format_args!(
						"cannot submit duplicate update for filter ({mode}, {runtype})",
						runtype = if teleports { "TP" } else { "Pro" },
					)));
				}
			}
		}

		Ok(Some(updates))
	}
}

/// A "raw" filter update.
///
/// Used during deserialization and in the public API.
/// The "real" type we work with though is [`FilterUpdate`].
#[derive(Debug, Deserialize, ToSchema)]
#[schema(as = FilterUpdate)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct RawFilterUpdate {
	mode: Mode,
	teleports: bool,

	#[serde(flatten)]
	#[schema(inline)]
	update: FilterUpdate,
}

/// An update to a course filter.
#[derive(Debug, Deserialize, ToSchema)]
pub struct FilterUpdate {
	/// A new tier.
	pub tier: Option<Tier>,

	/// A new ranked status.
	pub ranked_status: Option<RankedStatus>,

	/// New notes.
	pub notes: Option<Option<String>>,
}

impl FilterUpdate {
	/// Checks if a filter update is empty, i.e., only holds empty containers.
	pub const fn is_empty(&self) -> bool {
		let Self {
			tier,
			ranked_status,
			notes,
		} = self;

		tier.is_none() && ranked_status.is_none() && notes.is_none()
	}
}
