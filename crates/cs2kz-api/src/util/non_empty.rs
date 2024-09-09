use std::collections::{BTreeMap, BTreeSet};
use std::{fmt, ops};

use serde::{de, Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct NonEmpty<T>(T);

#[expect(private_bounds, reason = "`Length` is an implementation detail")]
impl<T> NonEmpty<T>
where
	T: Length,
{
	pub fn new(value: T) -> Option<Self>
	{
		if value.is_empty() {
			None
		} else {
			Some(Self(value))
		}
	}
}

impl<T> NonEmpty<T>
{
	pub fn as_ref(&self) -> NonEmpty<&T>
	{
		NonEmpty(&self.0)
	}

	pub fn as_deref(&self) -> NonEmpty<&<T as ops::Deref>::Target>
	where
		T: ops::Deref,
	{
		NonEmpty(self.0.deref())
	}

	pub fn into_inner(this: Self) -> T
	{
		this.0
	}
}

impl<T> fmt::Debug for NonEmpty<T>
where
	T: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Debug::fmt(&self.0, f)
	}
}

impl<T> fmt::Display for NonEmpty<T>
where
	T: fmt::Display,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(&self.0, f)
	}
}

impl<T> ops::Deref for NonEmpty<T>
{
	type Target = T;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl<T> IntoIterator for NonEmpty<T>
where
	T: IntoIterator,
{
	type Item = T::Item;
	type IntoIter = T::IntoIter;

	fn into_iter(self) -> Self::IntoIter
	{
		self.0.into_iter()
	}
}

impl<'a, T> IntoIterator for &'a NonEmpty<T>
where
	&'a T: IntoIterator,
{
	type Item = <&'a T as IntoIterator>::Item;
	type IntoIter = <&'a T as IntoIterator>::IntoIter;

	fn into_iter(self) -> Self::IntoIter
	{
		(&self.0).into_iter()
	}
}

impl<'de, T> Deserialize<'de> for NonEmpty<T>
where
	T: Deserialize<'de> + Length,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		T::deserialize(deserializer)
			.map(Self::new)
			.transpose()
			.ok_or_else(|| de::Error::invalid_length(0, &"1 or more"))?
	}
}

pub(super) trait Length
{
	fn length(&self) -> usize;

	fn is_empty(&self) -> bool
	{
		self.length() == 0
	}
}

impl<T> Length for Box<T>
where
	T: Length + ?Sized,
{
	fn length(&self) -> usize
	{
		<T>::length(&**self)
	}
}

impl<T> Length for [T]
{
	fn length(&self) -> usize
	{
		<[T]>::len(self)
	}
}

impl<T> Length for Vec<T>
{
	fn length(&self) -> usize
	{
		<[T]>::len(self)
	}
}

impl<K, V> Length for BTreeMap<K, V>
{
	fn length(&self) -> usize
	{
		<BTreeMap<K, V>>::len(self)
	}
}

impl<T> Length for BTreeSet<T>
{
	fn length(&self) -> usize
	{
		<BTreeSet<T>>::len(self)
	}
}

impl Length for str
{
	fn length(&self) -> usize
	{
		<str>::len(self)
	}
}

impl Length for String
{
	fn length(&self) -> usize
	{
		<str>::len(self)
	}
}
