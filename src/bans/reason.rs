use std::{cmp, time::Duration};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::time::DurationExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum BanReason
{
	Macro,
	AutoBhop,
	AutoStrafe,
}

impl BanReason
{
	pub fn ban_duration(&self, previous_ban_duration: Duration) -> Duration
	{
		let mut duration = match self {
			Self::Macro => Duration::WEEK * 2,
			Self::AutoBhop => Duration::MONTH,
			Self::AutoStrafe => Duration::MONTH * 2,
		};

		if !previous_ban_duration.is_zero() {
			duration = (duration + previous_ban_duration) * 2;
		}

		cmp::min(duration, Duration::YEAR)
	}
}
