use serde::{Deserialize, Serialize};

/// Bhop statistics.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct BhopStats
{
	/// Total bhop count.
	pub total: u32,

	/// The amount of "perfs" according to the mode.
	pub perfs: u32,

	/// The amount of tick-perfect bhops.
	pub perfect_perfs: u32,
}

impl BhopStats
{
	/// Checks if the stats are logically valid.
	pub fn is_valid(&self) -> bool
	{
		(self.total >= (self.perfs + self.perfect_perfs)) && (self.perfs <= self.perfect_perfs)
	}
}
