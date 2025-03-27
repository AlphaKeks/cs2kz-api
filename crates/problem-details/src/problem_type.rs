use std::fmt;

/// A problem type.
///
/// This can be used to customize the behavior of <code>[ProblemDetails]\<T></code> where <code>T:
/// [ProblemType]</code>.
///
/// [ProblemDetails]: crate::ProblemDetails
#[diagnostic::on_unimplemented(
	message = "`{Self}` is not an HTTP problem type",
	note = "only types that implement `ProblemType` can be used with `ProblemDetails<T>`"
)]
pub trait ProblemType
{
	/// The URI to encode in the response's [`type`] member.
	///
	/// [`type`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.1
	fn uri(&self) -> http::Uri;

	/// The status code to use in the response.
	///
	/// This is also the status code used in the response's [`status`] member.
	///
	/// [`status`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.2
	fn status(&self) -> http::StatusCode;

	/// The response's [`title`] member.
	///
	/// [`title`]: https://www.rfc-editor.org/rfc/rfc9457.html#section-3.1.3
	fn title(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result;
}
