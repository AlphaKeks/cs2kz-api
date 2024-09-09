//! This module implements functionality to get a specific KZ map by its ID or name.

use std::mem;

use cs2kz::{MapState, Mode, RankedStatus, SteamID, Tier};
use futures::{Stream, TryStreamExt};
use problem_details::AsProblemDetails;
use serde::Serialize;

use super::{Course, CourseID, FilterID, MapID, MapService, Mapper};
use crate::http::Problem;
use crate::services::steam::{MapFileHash, WorkshopID};
use crate::util::time::Timestamp;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

macro_rules! query {
	($extra:literal $(, $($args:tt)*)?) => {
		sqlx::query! {
			"SELECT
			   m.id `id: MapID`,
			   m.name,
			   m.description,
			   m.state `state: MapState`,
			   mm.name mapper_name,
			   mm.id `mapper_id: SteamID`,
			   m.workshop_id `workshop_id: WorkshopID`,
			   m.hash `hash: MapFileHash`,
			   c.id `course_id: CourseID`,
			   c.name course_name,
			   c.description course_description,
			   cm.name course_mapper_name,
			   cm.id `course_mapper_id: SteamID`,
			   f.id `filter_id: FilterID`,
			   f.game_mode `mode: Mode`,
			   f.has_teleports `has_teleports: bool`,
			   f.tier `tier: Tier`,
			   f.ranked_status `ranked_status: RankedStatus`,
			   f.notes filter_notes,
			   m.created_on `created_on: Timestamp`
			 FROM Maps m
			 JOIN Mappers ON Mappers.map_id = m.id
			 JOIN Users mm ON mm.id = Mappers.user_id
			 JOIN Courses c ON c.map_id = m.id
			 JOIN CourseMappers ON CourseMappers.course_id = c.id
			 JOIN Users cm ON cm.id = CourseMappers.user_id
			 JOIN CourseFilters f ON f.course_id = c.id "
			 + $extra,
			$($($args)*)?
		}
	};
}

macro_rules! map_row {
	() => {
		|mut row| $crate::services::maps::get_map::Response {
			id: row.id,
			name: row.name,
			description: row.description,
			state: row.state,
			mappers: vec![$crate::services::maps::Mapper {
				name: row.mapper_name,
				steam_id: row.mapper_id,
			}],
			workshop_id: row.workshop_id,
			hash: row.hash,
			courses: vec![$crate::services::maps::Course {
				id: row.course_id,
				name: row.course_name,
				description: row.course_description,
				mappers: vec![$crate::services::maps::Mapper {
					name: row.course_mapper_name,
					steam_id: row.course_mapper_id,
				}],
				filters: std::array::from_fn(|_| $crate::services::maps::Filter {
					id: row.filter_id,
					mode: row.mode,
					has_teleports: row.has_teleports,
					tier: row.tier,
					ranked_status: row.ranked_status,
					notes: std::mem::take(&mut row.filter_notes),
				}),
			}],
			created_on: row.created_on,
		}
	};
}

pub(super) use {map_row, query};

pub(super) fn reduce_result(result: &mut Response, mut curr: Response)
{
	sanity_check!(result.id == curr.id);

	result.mappers.append(&mut curr.mappers);

	let Some(course) = result
		.courses
		.iter_mut()
		.find(|c| c.id == curr.courses[0].id)
	else {
		result.courses.append(&mut curr.courses);
		return;
	};

	course.mappers.append(&mut curr.courses[0].mappers);

	let filter_slot = course
		.filters
		.iter_mut()
		.find(|f| f.id != curr.courses[0].filters[0].id)
		.expect("we haven't overwritten all filters yet");

	mem::swap(filter_slot, &mut curr.courses[0].filters[0]);
}

async fn aggregate_results<S>(mut stream: S) -> Result<Response>
where
	S: Stream<Item = sqlx::Result<Response>> + Send + Unpin,
{
	let Some(mut result) = stream.try_next().await? else {
		return Err(Error::MapNotFound);
	};

	while let Some(map) = stream.try_next().await? {
		reduce_result(&mut result, map);
	}

	sanity_check!(!result.mappers.is_empty());
	sanity_check!(!result.courses.is_empty());

	for course in &result.courses {
		sanity_check!(!course.mappers.is_empty());
		sanity_check!(course.filters[1..]
			.iter()
			.all(|f| f.id != course.filters[0].id));
	}

	Ok(result)
}

impl MapService
{
	/// Gets a map by its name.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_map_by_id(&self, map_id: MapID) -> Result
	{
		let rows = query!("WHERE m.id = ?", map_id)
			.fetch(&self.mysql)
			.map_ok(map_row!());

		aggregate_results(rows).await
	}

	/// Gets a map by its name.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_map_by_name(&self, map_name: &str) -> Result
	{
		let rows = query!("WHERE m.name LIKE ?", format!("%{map_name}%"))
			.fetch(&self.mysql)
			.map_ok(map_row!());

		aggregate_results(rows).await
	}

	/// Gets a map by its Steam workshop ID.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_map_by_workshop_id(&self, workshop_id: WorkshopID) -> Result
	{
		let rows = query!("WHERE m.workshop_id = ?", workshop_id)
			.fetch(&self.mysql)
			.map_ok(map_row!());

		aggregate_results(rows).await
	}
}

/// Response for getting a specific KZ map.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The map's ID.
	pub id: MapID,

	/// The map's name.
	pub name: String,

	/// A description of the map.
	pub description: String,

	/// The map's approval status.
	pub state: MapState,

	/// List of players who have contributed to making this map.
	pub mappers: Vec<Mapper>,

	/// The map's Steam workshop ID.
	pub workshop_id: WorkshopID,

	/// MD5 hash of the map file.
	pub hash: MapFileHash,

	/// Courses present on the map.
	pub courses: Vec<Course>,

	/// When this map was approved.
	pub created_on: Timestamp,
}

/// Errors that can occur when getting a map.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("map not found")]
	MapNotFound,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::MapNotFound => Problem::ResourceNotFound,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
