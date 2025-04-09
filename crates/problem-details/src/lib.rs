//! # [RFC 9457][rfc] - Problem Details for HTTP APIs
//!
//! This crate provides an implementation of [RFC 9457][rfc] that can be used with the [`http`]
//! crate and compatible frameworks.
//!
//! [rfc]: https://www.rfc-editor.org/rfc/rfc9457.html

#![feature(debug_closure_helpers)]
#![feature(decl_macro)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(unqualified_local_imports)]

pub use self::{extension_members::ExtensionMembers, problem_type::ProblemType};
use {
	mime::Mime,
	serde::ser::{Serialize, SerializeMap, Serializer},
	std::{any::type_name, borrow::Cow, fmt},
};

pub mod extension_members;
mod problem_type;

/// Returns the [`Content-Type`] value used in responses.
///
/// [`Content-Type`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Type
pub fn content_type() -> Mime
{
	"application/problem+json"
		.parse::<Mime>()
		.unwrap_or_else(|err| panic!("hard-coded string should always be valid: {err}"))
}

/// [RFC 9457][rfc] - Problem Details
///
/// [rfc]: https://www.rfc-editor.org/rfc/rfc9457.html
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProblemDetails<T: ProblemType>
{
	/// The problem type.
	///
	/// This corresponds to the [`type`] member in the response. This is generic so downstream
	/// users can choose their own problem types. The type you choose here should implement the
	/// [`ProblemType`] trait.
	///
	/// [`type`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.1
	problem_type: T,

	/// The response's [`detail`] member.
	///
	/// [`detail`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.4
	detail: Option<Cow<'static, str>>,

	/// The response's [`instance`] member.
	///
	/// [`instance`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.5
	instance: Option<Cow<'static, str>>,

	/// Additional fields to include in the response.
	///
	/// This corresponds to [Section 3.2] of the [RFC].
	///
	/// [Section 3.2]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.2
	/// [RFC]: https://www.rfc-editor.org/rfc/rfc9457.html
	extension_members: ExtensionMembers,
}

impl<T: ProblemType> ProblemDetails<T>
{
	/// Creates a new [`ProblemDetails`] object for the given [`ProblemType`].
	pub fn new(problem_type: T) -> Self
	{
		Self {
			problem_type,
			detail: None,
			instance: None,
			extension_members: ExtensionMembers::new(),
		}
	}

	/// Returns a shared reference to the [`ProblemType`] value.
	pub fn problem_type(&self) -> &T
	{
		&self.problem_type
	}

	/// Returns an exclusive reference to the [`ProblemType`] value.
	pub fn problem_type_mut(&mut self) -> &mut T
	{
		&mut self.problem_type
	}

	/// Returns the value of the [`detail`] field, if any.
	///
	/// [`detail`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.4
	pub fn detail(&self) -> Option<&str>
	{
		self.detail.as_deref()
	}

	/// Returns the value of the [`instance`] field, if any.
	///
	/// [`instance`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.5
	pub fn instance(&self) -> Option<&str>
	{
		self.instance.as_deref()
	}

	/// Returns a shared reference to the [`ExtensionMembers`].
	pub fn extension_members(&self) -> &ExtensionMembers
	{
		&self.extension_members
	}

	/// Returns an exclusive reference to the [`ExtensionMembers`].
	pub fn extension_members_mut(&mut self) -> &mut ExtensionMembers
	{
		&mut self.extension_members
	}

	/// Populates the [`detail`] field.
	///
	/// [`detail`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.4
	pub fn set_detail(&mut self, detail: impl Into<Cow<'static, str>>)
	{
		self.detail = Some(detail.into());
	}

	/// Populates the [`instance`] field.
	///
	/// [`instance`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.5
	pub fn set_instance(&mut self, instance: impl Into<Cow<'static, str>>)
	{
		self.instance = Some(instance.into());
	}

	/// Adds an [extension member] field.
	///
	/// # Panics
	///
	/// This function will panic if `value` cannot be serialized into a JSON value.
	///
	/// [extension member]: ExtensionMembers
	#[track_caller]
	pub fn add_extension_member<V>(&mut self, name: impl Into<String>, value: &V)
	where
		V: ?Sized + Serialize,
	{
		if let Err(error) = self.extension_members.add(name, value) {
			panic!("failed to serialize extension member of type `{}`: {error}", type_name::<V>());
		}
	}
}

impl<T: ProblemType> Serialize for ProblemDetails<T>
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let field_count = 3 // type + status + title
            + usize::from(self.detail().is_some())
            + usize::from(self.instance().is_some())
            + self.extension_members().count();

		let mut serializer = serializer.serialize_map(Some(field_count))?;

		serializer.serialize_entry("type", &format_args!("{}", self.problem_type().uri()))?;
		serializer.serialize_entry("status", &self.problem_type().status().as_u16())?;

		#[rustfmt::skip]
        serializer.serialize_entry("title", &format_args!("{}", fmt::from_fn(|fmt| {
            self.problem_type().title(fmt)
        })))?;

		if let Some(detail) = self.detail() {
			serializer.serialize_entry("detail", detail)?;
		}

		if let Some(instance) = self.instance() {
			serializer.serialize_entry("instance", instance)?;
		}

		for (key, value) in self.extension_members() {
			serializer.serialize_entry(key, value)?;
		}

		serializer.end()
	}
}

impl<T: ProblemType, B> From<ProblemDetails<T>> for http::Response<B>
where
	Vec<u8>: Into<B>,
{
	fn from(problem_details: ProblemDetails<T>) -> Self
	{
		(&problem_details).into()
	}
}

impl<T: ProblemType, B> From<&ProblemDetails<T>> for http::Response<B>
where
	Vec<u8>: Into<B>,
{
	fn from(problem_details: &ProblemDetails<T>) -> Self
	{
		let body = serde_json::to_vec(&problem_details).unwrap_or_else(|err| {
			panic!("failed to serialize `ProblemDetails<{}>` into JSON: {}", type_name::<T>(), err);
		});

		http::Response::builder()
			.status(problem_details.problem_type().status())
			.header(http::header::CONTENT_TYPE, content_type().as_ref())
			.body(body.into())
			.unwrap_or_else(|err| panic!("hard-coded response should be correct: {err}"))
	}
}

#[cfg(feature = "axum")]
impl<T: ProblemType> axum_core::response::IntoResponse for ProblemDetails<T>
{
	fn into_response(self) -> axum_core::response::Response
	{
		self.into()
	}
}

#[cfg(feature = "utoipa")]
mod utoipa_impls
{
	use {
		crate::{ProblemDetails, ProblemType},
		serde_json::json,
		std::borrow::Cow,
		utoipa::{
			PartialSchema,
			ToSchema,
			openapi::{
				RefOr,
				SchemaFormat,
				schema::{self, AdditionalProperties, Object, Schema},
			},
		},
	};

	impl<T: ProblemType + PartialSchema> ToSchema for ProblemDetails<T>
	{
		fn name() -> Cow<'static, str>
		{
			Cow::Borrowed("ProblemDetails")
		}
	}

	impl<T: ProblemType + PartialSchema> PartialSchema for ProblemDetails<T>
	{
		fn schema() -> RefOr<Schema>
		{
			macro get_first_example($obj:expr) {
				$obj.examples.first().map_or_else(Default::default, Clone::clone)
			}

			let example_type = match T::schema() {
				RefOr::T(Schema::Array(array)) => {
					get_first_example!(array)
				},
				RefOr::T(Schema::Object(object)) => {
					get_first_example!(object)
				},
				RefOr::T(Schema::OneOf(one_of)) => {
					get_first_example!(one_of)
				},
				RefOr::T(Schema::AllOf(all_of)) => {
					get_first_example!(all_of)
				},
				RefOr::T(Schema::AnyOf(any_of)) => {
					get_first_example!(any_of)
				},
				RefOr::Ref(_) | RefOr::T(_) => {
					"https://api.example.org/foo/bar/example-problem".into()
				},
			};

			let example = json!({
				"type": example_type,
				"status": 422_u16,
				"title": "something went wrong",
				"detail": "request body is invalid"
			});

			Object::builder()
				.description(Some("RFC 9457 - Problem Details for HTTP APIs"))
				.schema_type(schema::Type::Object)
				.format(Some(SchemaFormat::Custom(String::from("rfc-9457"))))
				.property("type", T::schema())
				.required("type")
				.property("status", u16::schema())
				.required("status")
				.property("title", str::schema())
				.required("title")
				.property("detail", str::schema())
				.property("instance", str::schema())
				.additional_properties(Some(AdditionalProperties::FreeForm(true)))
				.examples([example])
				.into()
		}
	}
}
