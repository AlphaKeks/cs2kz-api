//! This module contains the [`AsProblemDetails`] trait.

use std::borrow::Cow;

use crate::{ExtensionMembers, ProblemDetails};

/// A trait for errors that describe a failed HTTP request.
pub trait AsProblemDetails: std::error::Error {
	/// The problem type associated with this error.
	type ProblemType: crate::ProblemType;

	/// The problem type.
	fn problem_type(&self) -> Self::ProblemType;

	/// Adds extension members to a [`ProblemDetails`] instance being
	/// constructed.
	///
	/// This function is called by [`AsProblemDetails::as_problem_details`].
	/// Error types can use this to hook in and register any extra information
	/// they have.
	fn add_extension_members(&self, extension_members: &mut ExtensionMembers) {
		let _ = extension_members;
	}

	/// A human-readable explanation specific to this occurrence of the problem.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-detail>
	fn detail(&self) -> Cow<'static, str> {
		Cow::Owned(self.to_string())
	}

	/// Constructs a [`ProblemDetails`] from this error.
	fn as_problem_details(&self) -> ProblemDetails<Self::ProblemType> {
		let mut problem_details =
			ProblemDetails::new(self.problem_type()).with_detail(self.detail());

		self.add_extension_members(problem_details.extension_members_mut());

		problem_details
	}
}
