/// Creates a "drop guard" which will run the given function `f` on drop.
pub(crate) fn drop_guard<F>(f: F) -> impl Drop
where
	F: FnOnce(),
{
	struct DropGuard<F>
	where
		F: FnOnce(),
	{
		f: Option<F>,
	}

	impl<F> Drop for DropGuard<F>
	where
		F: FnOnce(),
	{
		fn drop(&mut self)
		{
			match self.f.take() {
				Some(f) => f(),
				None => unreachable!("`Drop::drop()` is only called once"),
			}
		}
	}

	DropGuard { f: Some(f) }
}
