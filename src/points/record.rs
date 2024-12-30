//! Record data relevant for calculating points.

use std::cmp;

use cs2kz::SteamID;

use crate::services::records::RecordID;

/// A single record.
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Record
{
	/// The record's ID.
	pub id: RecordID,

	/// SteamID of the player who set the record.
	pub player_id: SteamID,

	/// The time of the record.
	pub time: f64,
}

impl PartialEq for Record
{
	fn eq(&self, other: &Self) -> bool
	{
		self.id == other.id
	}
}

impl Eq for Record {}

impl PartialOrd for Record
{
	fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering>
	{
		Some(self.cmp(other))
	}
}

impl Ord for Record
{
	fn cmp(&self, other: &Self) -> cmp::Ordering
	{
		self.time.total_cmp(&other.time)
	}
}
