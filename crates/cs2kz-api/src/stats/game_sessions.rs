use std::collections::BTreeMap;
use std::iter;

use cs2kz::Mode;
use serde::Deserialize;

use crate::services::maps::CourseID;
use crate::stats::BhopStats;
use crate::util::time::Seconds;

make_id! {
	/// An ID uniquely identifying a game session.
	pub struct GameSessionID(u64);
}

make_id! {
	/// An ID uniquely identifying a course session.
	pub struct CourseSessionID(u64);
}

/// A game session.
///
/// A session begins when a player joins a server and ends when they disconnect.
#[derive(Debug, Deserialize)]
pub struct GameSession
{
	/// How many seconds the player was actively playing for.
	pub seconds_active: Seconds,

	/// How many seconds the player was spectating for.
	pub seconds_spectating: Seconds,

	/// How many seconds the player was inactive for.
	pub seconds_afk: Seconds,

	/// Bhop statistics for the entire session.
	pub bhop_stats: BhopStats,

	/// Per-course session information.
	pub course_sessions: BTreeMap<CourseID, CourseSession>,
}

impl GameSession
{
	/// Checks if the values in the session are logically valid.
	pub fn is_valid(&self) -> bool
	{
		self.bhop_stats.is_valid()
			&& self
				.course_sessions
				.values()
				.all(|session| session.is_valid())
	}
}

/// Session data for a specific course.
#[derive(Debug, Deserialize)]
pub struct CourseSession
{
	/// Data for this course in the vanilla mode.
	pub vanilla: CourseSessionData,

	/// Data for this course in the classic mode.
	pub classic: CourseSessionData,
}

impl CourseSession
{
	/// Checks if the values in the session are logically valid.
	pub fn is_valid(&self) -> bool
	{
		self.vanilla.is_valid() && self.classic.is_valid()
	}

	/// Creates an iterator over the data stored in this session.
	pub fn iter(&self) -> impl Iterator<Item = (Mode, &CourseSessionData)>
	{
		let vanilla = iter::once((Mode::Vanilla, &self.vanilla));
		let classic = iter::once((Mode::Classic, &self.classic));

		vanilla.chain(classic)
	}
}

/// Session data for a specific course+mode combination.
#[derive(Debug, Deserialize)]
pub struct CourseSessionData
{
	/// How much time the player spent with a running timer on this course+mode combination.
	pub playtime: Seconds,

	/// Bhop statistics for the entire session.
	pub bhop_stats: BhopStats,

	/// How many times the player left the start zone.
	pub started_runs: u16,

	/// How many times the player entered the end zone.
	pub finished_runs: u16,
}

impl CourseSessionData
{
	/// Checks if the values in the session are logically valid.
	pub fn is_valid(&self) -> bool
	{
		self.bhop_stats.is_valid() && (self.started_runs <= self.finished_runs)
	}
}
