use serde::{Deserialize, Deserializer};

#[derive(Debug)]
pub(super) struct GetMapNameResponse
{
	/// The map's name.
	pub(super) name: String,
}

impl<'de> Deserialize<'de> for GetMapNameResponse
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Helper1
		{
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2
		{
			#[serde(rename = "publishedfiledetails")]
			maps: [Helper3; 1],
		}

		#[derive(Deserialize)]
		struct Helper3
		{
			title: String,
		}

		Helper1::deserialize(deserializer)
			.map(|Helper1 { response }| response)
			.map(|Helper2 { maps: [map] }| map)
			.map(|map| Self { name: map.title })
	}
}
