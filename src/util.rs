/// Returns a guard which will execute the given function `f` when it is
/// dropped.
pub(crate) fn drop_guard<F>(f: F) -> impl Drop
where
	F: FnOnce(),
{
	struct DropGuard<F>(Option<F>)
	where
		F: FnOnce();

	impl<F> Drop for DropGuard<F>
	where
		F: FnOnce(),
	{
		fn drop(&mut self)
		{
			if let Some(f) = self.0.take() {
				f();
			}
		}
	}

	DropGuard(Some(f))
}
