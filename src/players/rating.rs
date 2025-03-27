use std::{cmp, num::FpCategory};

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Default, Clone, Copy, Serialize, sqlx::Type, ToSchema)]
#[debug("{_0:?}")]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PlayerRating(f64);

#[derive(Debug, Display, Error)]
#[display("invalid player rating: {_variant}")]
pub enum InvalidPlayerRating
{
	#[display("is NaN")]
	IsNaN,

	#[display("is ∞")]
	IsInfinity,

	#[display("is subnormal")]
	IsSubnormal,

	#[display("is negative")]
	IsNegative,
}

impl TryFrom<f64> for PlayerRating
{
	type Error = InvalidPlayerRating;

	fn try_from(value: f64) -> Result<Self, Self::Error>
	{
		match value.classify() {
			FpCategory::Nan => Err(InvalidPlayerRating::IsNaN),
			FpCategory::Infinite => Err(InvalidPlayerRating::IsInfinity),
			FpCategory::Subnormal => Err(InvalidPlayerRating::IsSubnormal),
			FpCategory::Zero | FpCategory::Normal if value.is_sign_negative() => {
				Err(InvalidPlayerRating::IsNegative)
			},
			FpCategory::Zero | FpCategory::Normal => {
				debug_assert!(value >= 0.0);
				Ok(Self(value))
			},
		}
	}
}

impl PartialEq for PlayerRating
{
	fn eq(&self, other: &Self) -> bool
	{
		self.0 == other.0
	}
}

impl Eq for PlayerRating
{
}

impl PartialOrd for PlayerRating
{
	fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering>
	{
		Some(self.cmp(other))
	}
}

impl Ord for PlayerRating
{
	fn cmp(&self, other: &Self) -> cmp::Ordering
	{
		self.0.total_cmp(&other.0)
	}
}
