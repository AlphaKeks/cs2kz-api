use serde::Serialize;

/// A 0-indexed position on a leaderboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
#[schema(value_type = u64)]
pub struct Rank(pub(crate) usize);

impl_sqlx!(Rank => {
	Type as i64;
	Encode<'q> as i64 = |rank| {
		rank.0 as i64
	};
	Decode<'r> as i64 = |value| {
		value.try_into().map(Self)
	};
});
