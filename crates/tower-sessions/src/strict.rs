/// The level of strictness to apply when authenticating & authorizing requests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Strict
{
	/// All requests are allowed.
	///
	/// If authentication succeeds, information will still be available in the request
	/// extensions, but wrapped in an [`Option`].
	Lax,

	/// Force authentication on all requests.
	///
	/// This will cause unauthenticated requests to be rejected before calling the inner
	/// service. Session information will be included in the request extensions.
	RequireAuthentication,

	/// Force authentication **and** authorization on all requests.
	///
	/// This is the same as [`RequireAuthentication`], but it also requires passing
	/// authorization checks.
	///
	/// [`RequireAuthentication`]: Strict::RequireAuthentication
	#[default]
	RequireAuthorization,
}
