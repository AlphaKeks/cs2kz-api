use std::borrow::Cow;

use cookie::SameSite;

/// Options for constructing HTTP cookies.
#[derive(Debug, Clone)]
pub struct CookieOptions
{
	pub(crate) domain: Cow<'static, str>,
	pub(crate) path: Cow<'static, str>,
	pub(crate) secure: bool,
	pub(crate) http_only: bool,
	pub(crate) same_site: SameSite,
}

impl CookieOptions
{
	/// Creates new [`CookieOptions`].
	pub fn new(domain: impl Into<Cow<'static, str>>, path: impl Into<Cow<'static, str>>) -> Self
	{
		Self {
			domain: domain.into(),
			path: path.into(),
			secure: true,
			http_only: true,
			same_site: SameSite::Lax,
		}
	}

	/// Dictates the value of the `Secure` field.
	///
	/// This is `true` by default.
	pub fn secure(mut self, secure: bool) -> Self
	{
		self.secure = secure;
		self
	}

	/// Dictates the value of the `HttpOnly` field.
	///
	/// This is `true` by default.
	pub fn http_only(mut self, http_only: bool) -> Self
	{
		self.http_only = http_only;
		self
	}

	/// Dictates the value of the `SameSite` field.
	///
	/// This is `Lax` by default.
	pub fn same_site(mut self, same_site: SameSite) -> Self
	{
		self.same_site = same_site;
		self
	}
}
