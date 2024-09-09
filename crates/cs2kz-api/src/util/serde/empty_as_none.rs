use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::super::non_empty::Length;

#[expect(dead_code, reason = "might be used later")]
#[expect(private_bounds, reason = "`Length` is an implementation detail")]
pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
	T: Length + Serialize,
	S: Serializer,
{
	value
		.as_ref()
		.filter(|v| !v.is_empty())
		.serialize(serializer)
}

#[expect(dead_code, reason = "might be used later")]
#[expect(private_bounds, reason = "`Length` is an implementation detail")]
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
	T: Length + Deserialize<'de>,
	D: Deserializer<'de>,
{
	Option::<T>::deserialize(deserializer).map(|opt| opt.filter(|v| !v.is_empty()))
}
