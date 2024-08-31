/*
 * Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see https://www.gnu.org/licenses.
 */

//! [RFC 9457 - Problem Details for HTTP APIs](https://www.rfc-editor.org/rfc/rfc9457.html)

use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod problem_type;
pub use problem_type::ProblemType;

mod extension_members;
pub use extension_members::ExtensionMembers;

mod as_problem_details;
pub use as_problem_details::AsProblemDetails;

/// [RFC 9457 - Problem Details for HTTP APIs](https://www.rfc-editor.org/rfc/rfc9457.html)
#[derive(Debug, Clone, PartialEq)]
pub struct ProblemDetails<T>
{
	/// The problem type.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-type>
	problem_type: T,

	/// A human-readable explanation specific to this occurrence of the problem.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-detail>
	detail: Option<Cow<'static, str>>,

	/// Additional information that is specific to the [problem type].
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-extension-members>
	///
	/// [problem type]: ProblemDetails::problem_type
	extension_members: ExtensionMembers,
}

impl<T> ProblemDetails<T>
{
	/// Creates a new [`ProblemDetails`].
	pub fn new(problem_type: T) -> Self
	{
		Self {
			problem_type,
			detail: None,
			extension_members: Default::default(),
		}
	}

	/// Adds [detail] describing this particular problem.
	///
	/// [detail]: ProblemDetails::detail()
	pub fn with_detail(mut self, detail: impl Into<Cow<'static, str>>) -> Self
	{
		self.detail = Some(detail.into());
		self
	}

	/// The problem type.
	pub const fn problem_type(&self) -> &T
	{
		&self.problem_type
	}

	/// A human-readable explanation specific to this occurrence of the problem.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-detail>
	pub fn detail(&self) -> Option<&str>
	{
		self.detail.as_deref()
	}

	/// Additional information that is specific to the [problem type].
	///
	/// [problem type]: ProblemDetails::problem_type()
	pub fn extension_members(&self) -> &ExtensionMembers
	{
		&self.extension_members
	}

	/// Returns a mutable reference to the [extension members].
	///
	/// [extension members]: ProblemDetails::extension_members()
	pub fn extension_members_mut(&mut self) -> &mut ExtensionMembers
	{
		&mut self.extension_members
	}
}

impl<T> ProblemDetails<T>
where
	T: ProblemType,
{
	/// The HTTP status code that will be used for the response.
	pub fn status(&self) -> http::StatusCode
	{
		self.problem_type.status()
	}

	/// A short, human-readable summary of the problem type.
	pub fn title(&self) -> &str
	{
		self.problem_type.title()
	}
}

impl<T, B> From<ProblemDetails<T>> for http::Response<B>
where
	T: ProblemType,
	Vec<u8>: Into<B>,
{
	fn from(problem_details: ProblemDetails<T>) -> Self
	{
		let status = problem_details.status();
		let body = serde_json::to_vec(&problem_details)
			.map(Into::<B>::into)
			.expect("problem details should serialize as json");

		http::Response::builder()
			.status(status)
			.body(body)
			.expect("problem type should serialize as json")
	}
}

#[cfg(feature = "axum")]
impl<T> axum_core::response::IntoResponse for ProblemDetails<T>
where
	T: ProblemType,
{
	fn into_response(self) -> axum_core::response::Response
	{
		self.into()
	}
}

impl<T> Serialize for ProblemDetails<T>
where
	T: ProblemType,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		use serde::ser::SerializeMap;

		let field_count = 1 // type
			+ usize::from(self.detail().is_some())
			+ self.extension_members().len();

		let mut serializer = serializer.serialize_map(Some(field_count))?;

		serializer.serialize_entry(
			"type",
			&problem_type::SerializeProblemType::new(self.problem_type()),
		)?;

		if let Some(detail) = self.detail() {
			serializer.serialize_entry("detail", detail)?;
		}

		for (k, v) in self.extension_members().iter() {
			serializer.serialize_entry(k, v)?;
		}

		serializer.end()
	}
}

impl<'de, T> Deserialize<'de> for ProblemDetails<T>
where
	T: ProblemType,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Debug, Deserialize)]
		#[serde(bound(deserialize = "problem_type::DeserializeProblemType<T>: Deserialize<'de>"))]
		struct Helper<T>
		where
			T: ProblemType,
		{
			#[serde(rename = "type")]
			problem_type: problem_type::DeserializeProblemType<T>,

			#[serde(default)]
			detail: Option<String>,

			#[serde(flatten)]
			extension_members: ExtensionMembers,
		}

		Helper::deserialize(deserializer).map(|v| Self {
			problem_type: v.problem_type.0,
			detail: v.detail.map(Cow::Owned),
			extension_members: v.extension_members,
		})
	}
}
