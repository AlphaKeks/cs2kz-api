#![allow(unused_macros, unused_macro_rules)]

macro_rules! assert {
	($cond:expr) => {
		if !($cond) {
			anyhow::bail!(
				"assertion failed: `{}`",
				std::stringify!($cond),
			);
		}
	};
	($cond:expr, $msg:literal $(, $($fmt:tt)*)?) => {
		if !($cond) {
			anyhow::bail!(
				"assertion failed: `{}` ({})",
				std::stringify!($cond),
				std::format_args!($msg $(, $($fmt)*)?),
			);
		}
	};
}

macro_rules! assert_eq {
	($lhs:expr, $rhs:expr) => {
		if !($lhs == $rhs) {
			anyhow::bail!(
				"assertion failed: `{}` == `{}`\n  lhs: {:?}\n  rhs: {:?}",
				std::stringify!($lhs),
				std::stringify!($rhs),
				$lhs,
				$rhs,
			);
		}
	};
	($lhs:expr, $rhs:expr, $msg:literal $(, $($fmt:tt)*)?) => {
		if !($lhs == $rhs) {
			anyhow::bail!(
				"assertion failed: `{}` == `{}` ({})\n  lhs: {:?}\n  rhs: {:?}",
				std::stringify!($lhs),
				std::stringify!($rhs),
				std::format_args!($msg $(, $($fmt)*)?),
				$lhs,
				$rhs,
			);
		}
	};
}

macro_rules! assert_ne {
	($lhs:expr, $rhs:expr) => {
		if ($lhs == $rhs) {
			anyhow::bail!(
				"assertion failed: `{}` != `{}`\n  lhs: {:?}\n  rhs: {:?}",
				std::stringify!($lhs),
				std::stringify!($rhs),
				$lhs,
				$rhs,
			);
		}
	};
	($lhs:expr, $rhs:expr, $msg:literal $(, $($fmt:tt)*)?) => {
		if ($lhs == $rhs) {
			anyhow::bail!(
				"assertion failed: `{}` != `{}` ({})\n  lhs: {:?}\n  rhs: {:?}",
				std::stringify!($lhs),
				std::stringify!($rhs),
				std::format_args!($msg $(, $($fmt)*)?),
				$lhs,
				$rhs,
			);
		}
	};
}

macro_rules! assert_matches {
	($expr:expr, $pat:pat $(if $guard:expr)? $(,)?) => {
		if !(std::matches!($expr, $pat $($guard)?)) {
			anyhow::bail!(
				"assertion failed: `{}` does not match `{}`",
				std::stringify!($expr),
				std::stringify!($pat),
			);
		}
	};
	($expr:expr, $pat:pat $(if $guard:expr)?, $msg:literal $(, $($fmt:tt)*)?) => {
		if !(std::matches!($pat $($guard)?)) {
			anyhow::bail!(
				"assertion failed: `{}` does not match `{}` ({})",
				std::stringify!($expr),
				std::stringify!($pat),
				std::format_args!($msg $(, $($fmt)*)?),
			);
		}
	};
}

#[allow(unused_imports)] // these may be used later
pub(crate) use {assert, assert_eq, assert_matches, assert_ne};
