use std::sync::Arc;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

type Object = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = Object)]
pub struct PlayerPreferences(Arc<Object>);

impl<DB> sqlx::Type<DB> for PlayerPreferences
where
	DB: sqlx::Database,
	sqlx::types::Json<()>: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<sqlx::types::Json<()> as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<sqlx::types::Json<()> as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for PlayerPreferences
where
	DB: sqlx::Database,
	for<'a> sqlx::types::Json<&'a Object>: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		sqlx::types::Json(&*self.0).encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		sqlx::types::Json(&*self.0).produces()
	}

	fn size_hint(&self) -> usize
	{
		sqlx::types::Json(&*self.0).size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for PlayerPreferences
where
	DB: sqlx::Database,
	sqlx::types::Json<Object>: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		sqlx::types::Json::<Object>::decode(value)
			.map(|sqlx::types::Json(preferences)| Self(Arc::new(preferences)))
	}
}
