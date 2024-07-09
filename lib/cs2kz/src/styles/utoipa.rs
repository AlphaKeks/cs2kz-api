//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::OneOfBuilder;
use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
use utoipa::{IntoParams, ToSchema};

use crate::Styles;

impl<'s> ToSchema<'s> for Styles
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		(
			"Styles",
			Schema::OneOf(
				OneOfBuilder::new()
					.nullable(false)
					.example(Some("auto_bhop".into()))
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("Name"))
							.schema_type(SchemaType::String)
							.example(Some("auto_bhop".into()))
							.enum_values(Some(Styles::ALL))
							.build(),
					))
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("ID"))
							.schema_type(SchemaType::Integer)
							.example(Some(1.into()))
							.enum_values(Some((0..Styles::ALL.count_ones() + 1).map(|n| 1 << n)))
							.build(),
					))
					.build(),
			)
			.into(),
		)
	}
}

impl IntoParams for Styles
{
	fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
	{
		vec![
			ParameterBuilder::new()
				.parameter_in(parameter_in_provider().unwrap_or_default())
				.name("styles")
				.schema(Some(Self::schema().1))
				.build(),
		]
	}
}
