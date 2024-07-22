//! [`Iterator`]s for [`Permissions`].

use super::Permissions;

/// An iterator over the multiple permissions contained in a [`Permissions`]
/// bitflags set.
#[derive(Debug, Clone)]
pub struct Iter
{
	/// The bits that are left.
	bits: u64,

	/// The current bit index we are at.
	idx: u32,
}

impl Iter
{
	/// Creates a new [`Iter`].
	fn new(permissions: Permissions) -> Self
	{
		Self { bits: permissions.bits(), idx: 0 }
	}
}

impl Iterator for Iter
{
	type Item = &'static str;

	fn next(&mut self) -> Option<Self::Item>
	{
		if self.bits == 0 {
			return None;
		}

		if self.idx >= u64::BITS {
			return None;
		}

		while self.bits != 0 && self.idx < u64::BITS {
			if let Some(name) = Permissions::new(self.bits & (1 << self.idx)).name() {
				self.idx += 1;
				return Some(name);
			}

			self.idx += 1;
		}

		None
	}
}

impl IntoIterator for Permissions
{
	type Item = &'static str;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		Iter::new(self)
	}
}

impl IntoIterator for &Permissions
{
	type Item = &'static str;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		Iter::new(*self)
	}
}
