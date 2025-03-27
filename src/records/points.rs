use std::{cmp, num::FpCategory};

use serde::Serialize;
use utoipa::ToSchema;

const MAX: f64 = 10_000.0;

#[derive(Debug, Default, Clone, Copy, Serialize, ToSchema, sqlx::Type)]
#[debug("{_0:?}")]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Points(f64);

#[derive(Debug, Display, Error)]
#[display("invalid points value: {_variant}")]
pub enum InvalidPoints
{
	#[display("is NaN")]
	IsNaN,

	#[display("is ∞")]
	IsInfinity,

	#[display("is subnormal")]
	IsSubnormal,

	#[display("is negative")]
	IsNegative,

	#[display("bigger than {MAX}")]
	TooBig,
}

impl Points
{
	/// The maximum points for any record.
	pub const MAX: Self = Self(MAX);

	pub fn as_f64(self) -> f64
	{
		self.0
	}
}

impl TryFrom<f64> for Points
{
	type Error = InvalidPoints;

	fn try_from(value: f64) -> Result<Self, Self::Error>
	{
		match value.classify() {
			FpCategory::Nan => Err(InvalidPoints::IsNaN),
			FpCategory::Infinite => Err(InvalidPoints::IsInfinity),
			FpCategory::Subnormal => Err(InvalidPoints::IsSubnormal),
			FpCategory::Zero | FpCategory::Normal if value.is_sign_negative() => {
				Err(InvalidPoints::IsNegative)
			},
			FpCategory::Normal if value > MAX => Err(InvalidPoints::TooBig),
			FpCategory::Zero | FpCategory::Normal => {
				debug_assert!(value >= 0.0);
				Ok(Self(value))
			},
		}
	}
}

impl PartialEq for Points
{
	fn eq(&self, other: &Self) -> bool
	{
		self.0 == other.0
	}
}

impl Eq for Points
{
}

impl PartialOrd for Points
{
	fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering>
	{
		Some(self.cmp(other))
	}
}

impl Ord for Points
{
	fn cmp(&self, other: &Self) -> cmp::Ordering
	{
		self.0.total_cmp(&other.0)
	}
}
