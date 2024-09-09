use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A duration in seconds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seconds(pub std::time::Duration);

impl Seconds
{
	pub fn new(secs: f64) -> Self
	{
		Self(std::time::Duration::from_secs_f64(secs))
	}
}

impl From<std::time::Duration> for Seconds
{
	fn from(duration: std::time::Duration) -> Self
	{
		Self(duration)
	}
}

impl From<Seconds> for std::time::Duration
{
	fn from(Seconds(duration): Seconds) -> Self
	{
		duration
	}
}

impl Serialize for Seconds
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.as_secs_f64().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		f64::deserialize(deserializer).map(Self::new)
	}
}

sql_type!(Seconds as f64 => {
	encode_by_ref: |self| &self.0.as_secs_f64(),
	encode: |self| self.0.as_secs_f64(),
	decode: |value| Ok(Self::new(value)),
});

macro_rules! impl_add {
	($lhs:ty, $rhs:ty) => {
		impl ::core::ops::Add<$rhs> for $lhs
		{
			type Output = $lhs;

			fn add(self, rhs: $rhs) -> Self::Output
			{
				self + rhs.0
			}
		}

		impl ::core::ops::AddAssign<$rhs> for $lhs
		{
			fn add_assign(&mut self, rhs: $rhs)
			{
				*self += rhs.0;
			}
		}
	};
}

impl_add!(std::time::Duration, Seconds);
impl_add!(std::time::Instant, Seconds);
impl_add!(tokio::time::Instant, Seconds);
