//! This module implements functionality to update KZ maps.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;

use cs2kz::{MapState, RankedStatus, SteamID, Tier};
use futures::{TryFutureExt, TryStreamExt};
use problem_details::AsProblemDetails;
use serde::Deserialize;

use super::{submit_map, CourseID, FilterID, MapID, MapService};
use crate::database::{self, QueryBuilder};
use crate::http::Problem;
use crate::services::steam::{self, WorkshopID};
use crate::services::SteamService;
use crate::util::NonEmpty;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl MapService
{
	/// Updates a map.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn update_map(&self, map_id: MapID, request: Request) -> Result
	{
		let mut txn = self.mysql.begin().await?;

		update_metadata(
			map_id,
			request.new_description.as_deref(),
			request.new_workshop_id,
			request.new_state,
			&self.steam_service,
			&mut txn,
		)
		.await?;

		update_mappers(
			map_id,
			request.added_mappers.as_ref().map(NonEmpty::as_ref),
			request.removed_mappers.as_ref().map(NonEmpty::as_ref),
			&mut txn,
		)
		.await?;

		if let Some(course_updates) = &request.course_updates {
			update_courses(map_id, course_updates.as_ref(), &mut txn).await?;
		}

		txn.commit().await?;

		info!(%map_id, "updated map");

		Ok(())
	}
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn update_metadata(
	map_id: MapID,
	new_description: Option<&str>,
	new_workshop_id: Option<WorkshopID>,
	new_state: Option<MapState>,
	steam_service: &SteamService,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let (name, hash) = match new_workshop_id {
		None => (None, None),
		Some(workshop_id) => tokio::try_join![
			steam_service
				.get_map_name(workshop_id)
				.map_err(Error::GetMapName),
			steam_service
				.download_map(workshop_id)
				.map_err(Error::DownloadMap)
				.and_then(|mut map| async move { map.hash().map_err(Error::HashMapFile).await }),
		]
		.map(|(name, hash)| (Some(name), Some(hash)))?,
	};

	let result = sqlx::query! {
		"UPDATE Maps
		 SET name = COALESCE(?, name),
		     hash = COALESCE(?, hash),
		     description = COALESCE(?, description),
		     workshop_id = COALESCE(?, workshop_id),
		     state = COALESCE(?, state)
		 WHERE id = ?",
		name,
		hash.as_deref(),
		new_description,
		new_workshop_id,
		new_state,
		map_id,
	}
	.execute(txn.as_mut())
	.await?;

	match result.rows_affected() {
		0 => return Err(Error::MapNotFound),
		n => sanity_check!(n == 1),
	}

	debug!(%map_id, "updated map metadata");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn update_mappers(
	map_id: MapID,
	added_mappers: Option<NonEmpty<&BTreeSet<SteamID>>>,
	removed_mappers: Option<NonEmpty<&BTreeSet<SteamID>>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	if let Some(&user_id) = Option::zip(added_mappers, removed_mappers)
		.map(|(added, removed)| added.intersection(*removed))
		.and_then(|mut overlap| overlap.next())
	{
		return Err(Error::AddAndRemoveMapper { user_id });
	}

	if let Some(mappers) = added_mappers {
		submit_map::insert_mappers(map_id, mappers, txn)
			.await
			.map_err(|error| match error {
				submit_map::Error::Database(error) => Error::Database(error),
				_ => unreachable!(),
			})?;
	}

	if let Some(mappers) = removed_mappers {
		remove_mappers(map_id, mappers, txn).await?;
		ensure_at_least_one_mapper(map_id, txn).await?;
	}

	debug!(%map_id, "updated mappers");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn remove_mappers(
	map_id: MapID,
	mappers: NonEmpty<&BTreeSet<SteamID>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mut query = QueryBuilder::new("DELETE FROM Mappers WHERE map_id = ");

	query.push_bind(map_id);
	query.push(" AND user_id IN (");

	let mut separated = query.separated(", ");

	for mapper in mappers {
		separated.push_bind(mapper);
	}

	separated.push_unseparated(")");

	let result = query.build().execute(txn.as_mut()).await?;

	if let Some(amount) = (mappers.len() as u64).checked_sub(result.rows_affected()) {
		warn!(amount, "unrecognized mappers in deletion request");
	}

	trace!(%map_id, ?mappers, "removed mappers for map");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn ensure_at_least_one_mapper(
	map_id: MapID,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mappers_count =
		sqlx::query_scalar!("SELECT COUNT(*) FROM Mappers WHERE map_id = ?", map_id)
			.fetch_one(txn.as_mut())
			.await?;

	if mappers_count == 0 {
		return Err(Error::ZeroMappers { is_map: true });
	}

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn update_courses(
	map_id: MapID,
	course_updates: NonEmpty<&BTreeMap<CourseID, CourseUpdate>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let valid_course_ids = sqlx::query_scalar! {
		"SELECT c.id `id: CourseID`
		 FROM Courses c
		 WHERE c.map_id = ?",
		map_id,
	}
	.fetch(txn.as_mut())
	.try_collect::<HashSet<_>>()
	.await?;

	if let Some(&course_id) = course_updates
		.keys()
		.find(|&id| !valid_course_ids.contains(id))
	{
		return Err(Error::InvalidCourseID { course_id });
	}

	for (&course_id, update) in course_updates {
		update_course_metadata(
			course_id,
			update.new_name.as_deref(),
			update.new_description.as_deref(),
			txn,
		)
		.await?;

		if let Some(mappers) = &update.added_mappers {
			submit_map::insert_course_mappers(course_id, mappers.as_ref(), txn)
				.await
				.map_err(|error| match error {
					submit_map::Error::Database(error) => Error::Database(error),
					_ => unreachable!(),
				})?;
		}

		if let Some(mappers) = &update.removed_mappers {
			remove_course_mappers(course_id, mappers.as_ref(), txn).await?;
			ensure_at_least_one_course_mapper(course_id, txn).await?;
		}

		if let Some(updates) = &update.filter_updates {
			update_filters(course_id, updates.as_ref(), txn).await?;
		}

		debug!(%course_id, "updated course");
	}

	debug!(%map_id, "updated courses");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn update_course_metadata(
	course_id: CourseID,
	new_name: Option<&str>,
	new_description: Option<&str>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	if new_name.is_none() && new_description.is_none() {
		return Ok(());
	}

	sqlx::query! {
		"UPDATE Courses
		 SET name = COALESCE(?, name),
		     description = COALESCE(?, description)
		 WHERE id = ?",
		course_id,
		new_name,
		new_description,
	}
	.execute(txn.as_mut())
	.await?;

	debug!(%course_id, "updated course metadata");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn remove_course_mappers(
	course_id: CourseID,
	mappers: NonEmpty<&BTreeSet<SteamID>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mut query = QueryBuilder::new("DELETE FROM CourseMappers WHERE course_id = ");

	query.push_bind(course_id);
	query.push(" AND user_id IN (");

	let mut separated = query.separated(", ");

	for mapper in mappers {
		separated.push_bind(mapper);
	}

	separated.push_unseparated(")");

	let result = query.build().execute(txn.as_mut()).await?;

	if let Some(amount) = (mappers.len() as u64).checked_sub(result.rows_affected()) {
		warn!(amount, "unrecognized mappers in deletion request");
	}

	trace!(%course_id, ?mappers, "removed mappers for course");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn ensure_at_least_one_course_mapper(
	course_id: CourseID,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let mappers_count = sqlx::query_scalar!(
		"SELECT COUNT(*) FROM CourseMappers WHERE course_id = ?",
		course_id
	)
	.fetch_one(txn.as_mut())
	.await?;

	if mappers_count == 0 {
		return Err(Error::ZeroMappers { is_map: false });
	}

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn update_filters(
	course_id: CourseID,
	filter_updates: NonEmpty<&BTreeMap<FilterID, FilterUpdate>>,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	let valid_filter_ids = sqlx::query_scalar! {
		"SELECT f.id `id: FilterID`
		 FROM CourseFilters f
		 WHERE f.course_id = ?",
		course_id,
	}
	.fetch(txn.as_mut())
	.try_collect::<HashSet<_>>()
	.await?;

	if let Some(&filter_id) = filter_updates
		.keys()
		.find(|&id| !valid_filter_ids.contains(id))
	{
		return Err(Error::InvalidFilterID {
			course_id,
			filter_id,
		});
	}

	for (&filter_id, update) in filter_updates {
		update_filter(filter_id, update, txn).await?;
	}

	debug!(%course_id, "updated course filters");

	Ok(())
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn update_filter(
	filter_id: FilterID,
	update: &FilterUpdate,
	txn: &mut database::Transaction<'_>,
) -> Result<()>
{
	sqlx::query! {
		"UPDATE CourseFilters
		 SET tier = COALESCE(?, tier),
		     ranked_status = COALESCE(?, ranked_status),
		     notes = COALESCE(?, notes)
		 WHERE id = ?",
		filter_id,
		update.new_tier,
		update.new_ranked_status,
		update.new_notes.as_deref(),
	}
	.execute(txn.as_mut())
	.await?;

	debug!(%filter_id, "updated course filter");

	Ok(())
}

/// Request for updating a KZ map.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// A new description for the map.
	pub new_description: Option<String>,

	/// A new workshop ID.
	///
	/// Including this field will cause the API to fetch up-to-date information about the map
	/// from Steam's workshop, as well as download it. This means that even if the workshop ID
	/// hasn't actually changed, this field can be used to update the map's name and hash.
	pub new_workshop_id: Option<WorkshopID>,

	/// A new approval status.
	pub new_state: Option<MapState>,

	/// SteamIDs of players to add as mappers of this map.
	pub added_mappers: Option<NonEmpty<BTreeSet<SteamID>>>,

	/// SteamIDs of players to remove as mappers of this map.
	pub removed_mappers: Option<NonEmpty<BTreeSet<SteamID>>>,

	/// Updates for individual courses.
	///
	/// Note that while these are keyed by course ID, you are only allowed to specify IDs of
	/// courses that actually belong to this map! If you specify unrelated IDs, the request
	/// will fail.
	pub course_updates: Option<NonEmpty<BTreeMap<CourseID, CourseUpdate>>>,
}

/// An update to a course.
#[derive(Debug, Deserialize)]
pub struct CourseUpdate
{
	/// A new name for the course.
	pub new_name: Option<String>,

	/// A new description for the course.
	pub new_description: Option<String>,

	/// SteamIDs of players to add as mappers of this course.
	pub added_mappers: Option<NonEmpty<BTreeSet<SteamID>>>,

	/// SteamIDs of players to remove as mappers of this course.
	pub removed_mappers: Option<NonEmpty<BTreeSet<SteamID>>>,

	/// Updates for individual filters.
	///
	/// Note that while these are keyed by filter ID, you are only allowed to specify IDs of
	/// filters that actually belong to this course! If you specify unrelated IDs, the request
	/// will fail.
	pub filter_updates: Option<NonEmpty<BTreeMap<FilterID, FilterUpdate>>>,
}

/// An update to a course filter.
#[derive(Debug, Deserialize)]
pub struct FilterUpdate
{
	/// A new tier for the filter.
	pub new_tier: Option<Tier>,

	/// A new ranked status for the filter.
	pub new_ranked_status: Option<RankedStatus>,

	/// New notes for the filter.
	pub new_notes: Option<String>,
}

/// Response for updating a KZ map.
pub type Response = ();

/// Errors that can occur when updating a map.
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

	#[error("map not found")]
	MapNotFound,

	#[error("cannot add and remove `{user_id}` simultaneously")]
	AddAndRemoveMapper
	{
		user_id: SteamID
	},

	#[error("every {} has to have at least one mapper", match is_map {
		true => "map",
		false => "course",
	})]
	ZeroMappers
	{
		is_map: bool
	},

	#[error("course {course_id} does not belong to this map")]
	InvalidCourseID
	{
		course_id: CourseID
	},

	#[error("filter {filter_id} does not belong to course {course_id}")]
	InvalidFilterID
	{
		course_id: CourseID,
		filter_id: FilterID,
	},

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
			Self::AddAndRemoveMapper { .. } => Problem::AddAndRemoveMapper,
			Self::ZeroMappers { .. } => Problem::ZeroMappers,
			Self::InvalidCourseID { .. } => Problem::InvalidCourseID,
			Self::InvalidFilterID { .. } => Problem::InvalidFilterID,
			Self::MapNotFound => Problem::ResourceNotFound,
			Self::DownloadMap(_) | Self::HashMapFile(_) | Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
