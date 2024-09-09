//! This module implements functionality to submit new KZ maps.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write;
use std::io;

use cs2kz::{MapState, Mode, RankedStatus, SteamID, Tier};
use futures::TryFutureExt;
use problem_details::AsProblemDetails;
use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::Row;

use super::{CourseID, MapID, MapService};
use crate::database::{self, ErrorExt, QueryBuilder};
use crate::http::Problem;
use crate::services::steam::{self, WorkshopID};
use crate::services::SteamService;
use crate::util::NonEmpty;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl MapService
{
	/// Submits a new map.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn submit_map(&self, request: Request) -> Result
	{
		let mut txn = self.mysql.begin().await?;

		let map_id = insert_map(
			&request.description,
			request.workshop_id,
			request.state,
			&self.steam_service,
			&mut txn,
		)
		.await?;

		insert_mappers(map_id, request.mappers.as_ref(), &mut txn).await?;
		insert_courses(map_id, request.courses.as_deref(), &mut txn).await?;

		txn.commit().await?;

		Ok(Response { map_id })
	}
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn insert_map(
	description: &str,
	workshop_id: WorkshopID,
	state: MapState,
	steam_service: &SteamService,
	txn: &mut database::Transaction<'_>,
) -> Result<MapID>
{
	let (name, hash) = tokio::try_join![
		steam_service
			.get_map_name(workshop_id)
			.map_err(Error::GetMapName),
		steam_service
			.download_map(workshop_id)
			.map_err(Error::DownloadMap)
			.and_then(|mut map| async move { map.hash().map_err(Error::HashMapFile).await }),
	]?;

	let deglobal_result = sqlx::query! {
		"UPDATE Maps
		 SET state = ?
		 WHERE name = ?",
		MapState::NotGlobal,
		name,
	}
	.execute(txn.as_mut())
	.await?;

	match deglobal_result.rows_affected() {
		0 => {}
		1 => debug!("degloballed old version of {name}"),
		n => warn!("degloballed {n} old versions of {name}"),
	}

	let map_id = sqlx::query! {
		"INSERT INTO Maps
		   (name, description, workshop_id, state, hash)
		 VALUES
		   (?, ?, ?, ?, ?)
		 RETURNING id",
		name,
		description,
		workshop_id,
		state,
		&*hash,
	}
	.fetch_one(txn.as_mut())
	.await
	.and_then(|row| row.try_get(0))?;

	info!(%map_id, "inserted map");

	Ok(map_id)
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
pub(super) async fn insert_mappers(
	map_id: MapID,
	mappers: NonEmpty<&BTreeSet<SteamID>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mut query = QueryBuilder::new("INSERT INTO Mappers (user_id, map_id) ");

	query.push_values(mappers, |mut query, user_id| {
		query.push_bind(user_id).push_bind(map_id);
	});

	query.build().execute(txn.as_mut()).await.map_err(|error| {
		if error.is_fk_violation("user_id") {
			Error::UnknownMapper
		} else {
			Error::Database(error)
		}
	})?;

	info!("inserted mappers");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn insert_courses(
	map_id: MapID,
	courses: NonEmpty<&[Course]>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	for course in courses {
		let course_id = sqlx::query! {
			"INSERT INTO Courses
			   (map_id, name, description)
			 VALUES
			   (?, ?, ?)
			 RETURNING id",
			map_id,
			&**course.name,
			course.description,
		}
		.fetch_one(txn.as_mut())
		.await
		.and_then(|row| row.try_get(0))?;

		insert_course_mappers(course_id, course.mappers.as_ref(), txn).await?;
		insert_course_filters(course_id, &course.filters, txn).await?;
	}

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
pub(super) async fn insert_course_mappers(
	course_id: CourseID,
	mappers: NonEmpty<&BTreeSet<SteamID>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mut query = QueryBuilder::new("INSERT INTO CourseMappers (user_id, course_id) ");

	query.push_values(mappers, |mut query, user_id| {
		query.push_bind(user_id).push_bind(course_id);
	});

	query.build().execute(txn.as_mut()).await.map_err(|error| {
		if error.is_fk_violation("user_id") {
			Error::UnknownMapper
		} else {
			Error::Database(error)
		}
	})?;

	info!("inserted course mappers");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn insert_course_filters(
	course_id: CourseID,
	filters: &[CourseFilter; 4],
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mut query = QueryBuilder::new(
		"INSERT INTO CourseFilters
		   (course_id, game_mode, has_teleports, tier, ranked_status, notes) ",
	);

	query.push_values(filters, |mut query, filter| {
		query
			.push_bind(course_id)
			.push_bind(filter.mode)
			.push_bind(filter.has_teleports)
			.push_bind(filter.tier)
			.push_bind(filter.ranked_status)
			.push_bind(&filter.notes);
	});

	query.build().execute(txn.as_mut()).await?;

	info!("inserted course filters");

	Ok(())
}

/// Request for submitting a new KZ map.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// A description of the map.
	#[serde(default)]
	pub description: String,

	/// The map's Steam workshop ID.
	pub workshop_id: WorkshopID,

	/// The initial approval status.
	pub state: MapState,

	/// SteamIDs of the people who contributed to making the map.
	pub mappers: NonEmpty<BTreeSet<SteamID>>,

	/// Courses present on the map.
	#[serde(deserialize_with = "Request::deserialize_courses")]
	pub courses: NonEmpty<Vec<Course>>,
}

impl Request
{
	/// Deserializes and validates courses.
	fn deserialize_courses<'de, D>(deserializer: D) -> Result<NonEmpty<Vec<Course>>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let courses = NonEmpty::<Vec<Course>>::deserialize(deserializer)?;
		let mut names = HashSet::with_capacity(courses.len());

		if let Some(duplicate) = courses
			.iter()
			.map(|course| &**course.name)
			.find(|&name| !names.insert(name))
		{
			return Err(de::Error::custom(format!(
				"duplicate course name `{duplicate}`"
			)));
		}

		Ok(courses)
	}
}

/// A KZ map course.
#[derive(Debug, Deserialize)]
pub struct Course
{
	/// The course's name.
	pub name: NonEmpty<String>,

	/// A description of the course.
	#[serde(default)]
	pub description: String,

	/// SteamIDs of the people who contributed to making the course.
	pub mappers: NonEmpty<BTreeSet<SteamID>>,

	/// The course filters.
	#[serde(deserialize_with = "Course::deserialize_filters")]
	pub filters: [CourseFilter; 4],
}

impl Course
{
	/// Deserializes and validates course filters.
	fn deserialize_filters<'de, D>(deserializer: D) -> Result<[CourseFilter; 4], D::Error>
	where
		D: Deserializer<'de>,
	{
		let filters = <[CourseFilter; 4]>::deserialize(deserializer)?;
		let mut seen = HashSet::from([
			(Mode::Vanilla, false),
			(Mode::Vanilla, true),
			(Mode::Classic, false),
			(Mode::Classic, true),
		]);

		if let Some((mode, has_teleports)) = filters
			.iter()
			.map(|filter| (filter.mode, filter.has_teleports))
			.find(|filter| !seen.remove(filter))
		{
			return Err(de::Error::custom(format!(
				"duplicate filter ({mode} {})",
				if has_teleports { "TP" } else { "Pro" },
			)));
		}

		let mut missing = seen.into_iter();

		if let Some((mode, has_teleports)) = missing.next() {
			let mut error = format!("{mode} {}", match has_teleports {
				true => "TP",
				false => "Pro",
			});

			for (mode, has_teleports) in missing {
				_ = write!(&mut error, ", {mode} {}", match has_teleports {
					true => "TP",
					false => "Pro",
				});
			}

			return Err(de::Error::custom(format!("missing filter(s): {error}")));
		}

		Ok(filters)
	}
}

/// A KZ map course filter.
#[derive(Debug, Deserialize)]
pub struct CourseFilter
{
	/// The filter's mode.
	pub mode: Mode,

	/// Whether this filter applies to runs with or without teleports.
	pub has_teleports: bool,

	/// The filter's tier.
	pub tier: Tier,

	/// The filter's ranked status.
	pub ranked_status: RankedStatus,

	/// Notes about this filter.
	#[serde(default)]
	pub notes: String,
}

/// Response for submitting a new KZ map.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The ID generated for the map.
	pub map_id: MapID,
}

/// Errors that can occur when submitting a new map.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("failed to get map information from Steam")]
	GetMapName(#[from] steam::GetMapNameError),

	#[error("something went wrong; please report this incident")]
	DownloadMap(#[from] steam::DownloadMapError),

	#[error("something went wrong; please report this incident")]
	HashMapFile(io::Error),

	#[error("one of the provided mappers is unknown to us")]
	UnknownMapper,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::GetMapName(steam::GetMapNameError::Http(error)) => error
				.status()
				.filter(|status| status.is_client_error())
				.map_or(Problem::ExternalService, |_| Problem::InvalidWorkshopID),
			Self::UnknownMapper => Problem::UnknownMapper,
			Self::DownloadMap(_) | Self::HashMapFile(_) | Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
