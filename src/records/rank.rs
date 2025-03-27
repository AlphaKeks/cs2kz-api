use serde::Serialize;

/// A 0-indexed position on a leaderboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Rank(pub(crate) usize);
