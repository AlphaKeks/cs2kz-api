use {
	serde::{Deserialize, Deserializer, Serialize, Serializer},
	std::time::Duration,
	utoipa::ToSchema,
};

/// A wrapper around [`Duration`] that ensures encoding/decoding always happens
/// in terms of seconds
#[derive(
	Debug, Display, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Into, ToSchema,
)]
#[display("{:.2}s", _0.as_secs_f64())]
#[schema(value_type = f64, description = "A duration in seconds")]
pub struct Seconds(pub Duration);

impl Seconds
{
	pub const fn as_f64(self) -> f64
	{
		self.0.as_secs_f64()
	}
}

impl From<f64> for Seconds
{
	fn from(value: f64) -> Self
	{
		Self(Duration::from_secs_f64(value))
	}
}

impl Serialize for Seconds
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_f64().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		f64::deserialize(deserializer).map(Self::from)
	}
}

impl_sqlx!(Seconds => {
	Type as f64;
	Encode<'q> as f64 = |seconds| seconds.as_f64();
	Decode<'r> as f64 = |value| Ok::<_, !>(Seconds::from(value));
});
