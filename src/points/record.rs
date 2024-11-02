//! Per-record data.

use std::cmp;

use crate::services::records::RecordID;

/// Record data relevant for calculating points.
#[derive(Debug)]
#[cfg_attr(test, derive(serde::Deserialize))]
pub struct Record
{
	/// The record's ID.
	pub id: RecordID,

	/// The time it took the player to complete the course.
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
