//! Macros used by this crate.
//!
//! Public macros are annotated with `#[macro_export]` and are accessible to downstream crates.
//! All the other macros are automatically in-scope within this crate.

#![allow(unused_macros, unused_macro_rules)]

/// Asserts that a value matches a pattern.
///
/// # Example
///
/// ```ignore
/// let x = 5;
///
/// assert_matches!(x, 1..=10);
/// assert_matches!(x, 1..=10, "how?");
/// assert_matches!(x, 1..=10, "foo = {}", false);
/// ```
macro_rules! assert_matches {
	($expr:expr, $pat:pat $(if $guard:expr)? $(,)?) => {
		::std::assert!(
			::std::matches!($expr, $pat $($guard)?),
			"`{}` did not match `{}` (was `{:?}`)",
			::std::stringify!($expr),
			::std::stringify!($pat),
			$expr,
		)
	};
	($expr:expr, $pat:pat $(if $guard:expr)?, $msg:literal $(, $($fmt:tt)*)?) => {
		::std::assert!(
			::std::matches!($expr, $pat $($guard)?),
			$msg,
			$($($fmt)*)?
		)
	};
}

/// Asserts that a value matches a pattern, if debug assertions are enabled.
///
/// # Example
///
/// ```ignore
/// let x = 5;
///
/// debug_assert_matches!(x, 1..=10);
/// debug_assert_matches!(x, 1..=10, "how?");
/// debug_assert_matches!(x, 1..=10, "foo = {}", false);
/// ```
macro_rules! debug_assert_matches {
	($val:expr, $pat:pat) => {
		::std::debug_assert!(
			::std::matches!($val, $pat),
			"`{}` did not match `{}` (was `{:?}`)",
			::std::stringify!($val),
			::std::stringify!($pat),
			$val,
		)
	};
	($val:expr, $pat:pat, $fmt:literal $(, $($fmt_args:tt)*)?) => {
		::std::debug_assert!(
			::std::matches!($val, $pat),
			$fmt,
			$($($fmt_args)*)?
		)
	};
}

/// Enables items conditionally based on whether the `rand` feature is enabled.
macro_rules! cfg_rand {
	(
		$($item:item)*
	) => {
		$(
			#[cfg(feature = "rand")]
			#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
			$item
		)*
	};
}

/// Enables items conditionally based on whether the `serde` feature is enabled.
macro_rules! cfg_serde {
	(
		$($item:item)*
	) => {
		$(
			#[cfg(feature = "serde")]
			#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
			$item
		)*
	};
}

/// Enables items conditionally based on whether the `sqlx` feature is enabled.
macro_rules! cfg_sqlx {
	(
		$($item:item)*
	) => {
		$(
			#[cfg(feature = "sqlx")]
			#[cfg_attr(docsrs, doc(cfg(feature = "sqlx")))]
			$item
		)*
	};
}

/// Creates a [`SteamID`] at compile time.
///
/// # Examples
///
/// ```
/// use cs2kz::steam_id;
///
/// let my_id = steam_id!(76561198282622073);
///
/// assert_eq!(my_id.as_u64(), 76561198282622073_u64);
///
/// let also_my_id = steam_id!(322356345);
///
/// assert_eq!(also_my_id.as_u64(), 76561198282622073_u64);
/// ```
///
/// [`SteamID`]: crate::steam_id::SteamID
#[macro_export]
macro_rules! steam_id {
	($steam_id:literal) => {
		const {
			let steam_id: u64 = $steam_id;

			if (steam_id > (::std::primitive::u32::MAX as ::std::primitive::u64)) {
				match $crate::steam_id::SteamID::from_u64(steam_id) {
					Some(steam_id) => steam_id,
					None => panic!("literal is not a valid 64-bit SteamID"),
				}
			} else {
				match $crate::steam_id::SteamID::from_u32(steam_id as ::std::primitive::u32) {
					Some(steam_id) => steam_id,
					None => panic!("literal is not a valid 32-bit SteamID"),
				}
			}
		}
	};
}
