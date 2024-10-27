use std::time::Duration;
use std::{fmt, ops};

use time::format_description::well_known::Rfc3339;

/// A timestamp.
///
/// This is currently implemented via [`time::OffsetDateTime`], but if we ever
/// need to switch to something else, like the `chrono` crate, we can swap it
/// out without having to change any other code.
///
/// The public API of this type is:
///    1. A UTC timestamp of "now" can be captured using [`Timestamp::now()`].
///    2. Arithmetic can be performed on [`Timestamp`]s using [`Duration`].
///    3. Compatibility with crates like [`serde`] and [`sqlx`], ensuring the
///       RFC3339 format for encoding/decoding.
///
/// The last point is especially important, as it is not the default behavior of
/// [`time::OffsetDateTime`].
#[derive(
	Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(description = "an RFC3339 date-time")]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] time::OffsetDateTime);

impl Timestamp {
	/// Captures a UTC timestamp of "now".
	pub fn now() -> Self {
		Self(time::OffsetDateTime::now_utc())
	}
}

impl fmt::Debug for Timestamp {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let formatted = self
			.0
			.format(&Rfc3339)
			.expect("timestamps should be reasonable");

		write!(fmt, "{formatted}")
	}
}

impl fmt::Display for Timestamp {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, fmt)
	}
}

impl ops::Add<Duration> for Timestamp {
	type Output = Self;

	fn add(self, duration: Duration) -> Self::Output {
		Self(self.0 + duration)
	}
}

impl ops::Add<Timestamp> for Duration {
	type Output = Timestamp;

	fn add(self, Timestamp(timestamp): Timestamp) -> Self::Output {
		Timestamp(timestamp + self)
	}
}
