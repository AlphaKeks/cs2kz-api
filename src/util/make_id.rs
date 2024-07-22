//! Helper macro for creating "ID" types.

/// Creates a new "ID" type.
macro_rules! make_id {
	($(#[$meta:meta])* $name:ident as u64) => {
		$crate::util::make_id!(@private $(#[$meta])* $name as u64);
	};

	($(#[$meta:meta])* $name:ident as $repr:ty) => {
		$crate::util::make_id!(@private $(#[$meta])* $name as $repr);

		impl TryFrom<u64> for $name
		{
			type Error = <$repr as TryFrom<u64>>::Error;

			fn try_from(value: u64) -> std::result::Result<Self, Self::Error>
			{
				<$repr>::try_from(value).map(Self)
			}
		}
	};

	(@private $(#[$meta:meta])* $name:ident as $repr:ty) => {
		$(#[$meta])*
		#[repr(transparent)]
		#[derive(
			Debug,
			Clone,
			Copy,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			serde::Serialize,
			serde::Deserialize,
			sqlx::Type,
		)]
		#[serde(transparent)]
		#[sqlx(transparent)]
		pub struct $name(pub $repr);

		impl std::fmt::Display for $name
		{
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				std::fmt::Display::fmt(&self.0, f)
			}
		}

		impl std::ops::Deref for $name
		{
			type Target = $repr;

			fn deref(&self) -> &Self::Target
			{
				&self.0
			}
		}

		impl From<$name> for $repr
		{
			fn from(value: $name) -> Self
			{
				value.0
			}
		}

		impl From<$repr> for $name
		{
			fn from(value: $repr) -> Self
			{
				Self(value)
			}
		}

		impl std::str::FromStr for $name
		{
			type Err = <$repr as std::str::FromStr>::Err;

			fn from_str(s: &str) -> std::result::Result<Self, Self::Err>
			{
				<$repr as std::str::FromStr>::from_str(s).map(Self)
			}
		}
	};
}

pub(crate) use make_id;
