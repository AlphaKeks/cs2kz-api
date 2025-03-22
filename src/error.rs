use std::error::Error;

/// Extension trait for <code>[Result]\<T, E></code>
pub trait ResultExt
{
	type Ok;
	type Err;

	/// [`Result::inspect_err()`] but with the error cast to
	/// <code>[&][]dyn [Error]</code>
	fn inspect_err_dyn(self, inspect: impl FnOnce(&(dyn Error + 'static))) -> Self
	where
		Self::Err: Error + 'static;
}

impl<T, E> ResultExt for Result<T, E>
{
	type Ok = T;
	type Err = E;

	fn inspect_err_dyn(self, inspect: impl FnOnce(&(dyn Error + 'static))) -> Self
	where
		<Self as ResultExt>::Err: Error + 'static,
	{
		self.inspect_err(move |err| inspect(err as &(dyn Error + 'static)))
	}
}
