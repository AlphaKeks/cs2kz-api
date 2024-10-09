use crate::database;

#[derive(Debug, Display, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct PlayerName(String);

impl PlayerName {
	/// Creates a new [`PlayerName`].
	///
	/// If the provided `name` is empty, this function returns [`None`].
	pub fn new(name: impl Into<String>) -> Option<Self> {
		let name: String = name.into();

		if name.is_empty() {
			return None;
		}

		Some(Self(name))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl<'de> serde::Deserialize<'de> for PlayerName {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		let value = String::deserialize(deserializer)?;

		if value.is_empty() {
			return Err(de::Error::invalid_length(0, &"more than 0"));
		}

		Ok(Self(value))
	}
}

database::macros::wrap!(PlayerName as str => {
	get: |self| self.as_str();
	make: |value| PlayerName::new(value).ok_or_else(|| "invalid player name".into());
});
