//! Reasons for which players can get banned.

use std::time::Duration;
use std::{cmp, fmt};

use serde::{Deserialize, Serialize};

use crate::util::time::DurationExt;

/// Reasons for which players can get banned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum BanReason
{
	/// Automated tick-perfect bhops.
	AutoBhop,

	/// Automated perfect airstrafes.
	AutoStrafe,

	/// Some kind of macro to automate parts of movement.
	Macro,
}

impl BanReason
{
	/// Determines the duration for a ban given the ban reason.
	///
	/// `previous_ban_duration` represents the sum of the durations of all
	/// previous non-false bans.
	pub fn duration(&self, previous_ban_duration: Option<Duration>) -> Duration
	{
		let base_duration = match self {
			BanReason::AutoBhop => Duration::MONTH * 2,
			BanReason::AutoStrafe => Duration::MONTH,
			BanReason::Macro => Duration::WEEK * 2,
		};

		let final_duration =
			previous_ban_duration.map_or(base_duration, |duration| (base_duration + duration) * 2);

		cmp::max(Duration::YEAR, final_duration)
	}
}

impl fmt::Display for BanReason
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.write_str(match self {
			BanReason::AutoBhop => "auto_bhop",
			BanReason::AutoStrafe => "auto_strafe",
			BanReason::Macro => "macro",
		})
	}
}
