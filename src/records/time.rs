use {
	crate::time::Seconds,
	serde::{Deserialize, Serialize},
	std::{cmp, num::FpCategory},
	utoipa::ToSchema,
};

#[derive(
	Debug, Display, Default, Clone, Copy, From, Into, Serialize, Deserialize, ToSchema, sqlx::Type,
)]
#[debug("{_0:?}")]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Time(Seconds);

#[derive(Debug, Display, Error)]
#[display("invalid time value: {_variant}")]
pub enum InvalidTime
{
	#[display("is NaN")]
	IsNaN,

	#[display("is ∞")]
	IsInfinity,

	#[display("is subnormal")]
	IsSubnormal,

	#[display("is negative")]
	IsNegative,

	#[display("is zero")]
	IsZero,
}

impl Time
{
	pub const fn as_f64(self) -> f64
	{
		self.0.as_f64()
	}
}

impl TryFrom<f64> for Time
{
	type Error = InvalidTime;

	fn try_from(value: f64) -> Result<Self, Self::Error>
	{
		match value.classify() {
			FpCategory::Nan => Err(InvalidTime::IsNaN),
			FpCategory::Infinite => Err(InvalidTime::IsInfinity),
			FpCategory::Subnormal => Err(InvalidTime::IsSubnormal),
			FpCategory::Zero => Err(InvalidTime::IsZero),
			FpCategory::Normal if value.is_sign_negative() => Err(InvalidTime::IsNegative),
			FpCategory::Normal => {
				debug_assert!(value > 0.0);
				Ok(Self(value.into()))
			},
		}
	}
}

impl PartialEq for Time
{
	fn eq(&self, other: &Self) -> bool
	{
		self.as_f64() == other.as_f64()
	}
}

impl Eq for Time
{
}

impl PartialOrd for Time
{
	fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering>
	{
		Some(self.cmp(other))
	}
}

impl Ord for Time
{
	fn cmp(&self, other: &Self) -> cmp::Ordering
	{
		self.as_f64().total_cmp(&other.as_f64())
	}
}
