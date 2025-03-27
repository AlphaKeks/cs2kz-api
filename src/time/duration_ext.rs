pub trait DurationExt
{
	const HOUR: Self;
	const DAY: Self;
	const WEEK: Self;
	const MONTH: Self;
	const YEAR: Self;
}

impl DurationExt for std::time::Duration
{
	const HOUR: Self = Self::from_secs(60 * 60);
	const DAY: Self = Self::from_secs(60 * 60 * 24);
	const WEEK: Self = Self::from_secs(60 * 60 * 24 * 7);
	const MONTH: Self = Self::from_secs(60 * 60 * 24 * 7 * 30);
	const YEAR: Self = Self::from_secs(60 * 60 * 24 * 365);
}

impl DurationExt for time::Duration
{
	const HOUR: Self = Self::seconds(60 * 60);
	const DAY: Self = Self::seconds(60 * 60 * 24);
	const WEEK: Self = Self::seconds(60 * 60 * 24 * 7);
	const MONTH: Self = Self::seconds(60 * 60 * 24 * 7 * 30);
	const YEAR: Self = Self::seconds(60 * 60 * 24 * 365);
}
