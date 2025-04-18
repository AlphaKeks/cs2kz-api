use {
	serde::{Deserialize, Serialize},
	std::sync::Arc,
	utoipa::ToSchema,
};

type Object = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = Object)]
pub struct PlayerPreferences(Arc<Object>);

impl_sqlx!(PlayerPreferences => {
	Type as sqlx::types::Json<()>;
	Encode<'q, 'a> as sqlx::types::Json<&'a Object> = |preferences| {
		sqlx::types::Json(&*preferences.0)
	};
	Decode<'r> as sqlx::types::Json<Object> = |sqlx::types::Json(object)| {
		Ok::<_, !>(Self(Arc::new(object)))
	};
});
