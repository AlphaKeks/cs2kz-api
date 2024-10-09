//! Various utility macros.

use std::fmt;

// FIXME: remove when `std::assert_matches!` is stable
// copied from https://doc.rust-lang.org/1.81.0/src/core/macros/mod.rs.html#154-179,

/// Asserts that an expression matches the provided pattern.
///
/// This macro is generally preferable to `assert!(matches!(value, pattern))`, because it can print
/// the debug representation of the actual value shape that did not meet expectations. In contrast,
/// using [`assert!`] will only print that expectations were not met, but not why.
///
/// The pattern syntax is exactly the same as found in a match arm and the `matches!` macro. The
/// optional if guard can be used to add additional checks that must be true for the matched value,
/// otherwise this macro will panic.
///
/// On panic, this macro will print the value of the expression with its debug representation.
///
/// Like [`assert!`], this macro has a second form, where a custom panic message can be provided.
///
/// # Examples
///
/// ```
/// let a = Some(345);
/// let b = Some(56);
/// assert_matches!(a, Some(_));
/// assert_matches!(b, Some(_));
///
/// assert_matches!(a, Some(345));
/// assert_matches!(a, Some(345) | None);
///
/// // assert_matches!(a, None); // panics
/// // assert_matches!(b, Some(345)); // panics
/// // assert_matches!(b, Some(345) | None); // panics
///
/// assert_matches!(a, Some(x) if x > 100);
/// // assert_matches!(a, Some(x) if x < 100); // panics
/// ```
#[cfg(test)]
macro_rules! assert_matches {
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::macros::assert_matches_failed(
                    left_val,
                    ::std::stringify!($($pattern)|+ $(if $guard)?),
                    ::std::option::Option::None
                );
            }
        }
    };
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )?, $($arg:tt)+) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::macros::assert_matches_failed(
                    left_val,
                    ::std::stringify!($($pattern)|+ $(if $guard)?),
                    ::std::option::Option::Some(::std::format_args!($($arg)+))
                );
            }
        }
    };
}

macro_rules! impl_error_from {
	($from:ty => $into:ty => { $(
		$($pattern:pat_param)|+ $(if $guard:expr)? => $result:expr
	),* $(,)? }) => {
		impl From<$from> for $into {
			fn from(error: $from) -> Self {
				use $from as E;

				match error { $(
					$($pattern)|+ $(if $guard)? => $result
				),* }
			}
		}
	};
}

#[track_caller]
pub(crate) fn assert_matches_failed(
	left: &impl fmt::Debug,
	right: &str,
	args: Option<fmt::Arguments<'_>>,
) -> ! {
	match args {
		Some(args) => panic!(
			r#"assertion `left matches right` failed: {args}
  left: `{left:?}`
 right: `{right}`"#
		),
		None => panic!(
			r#"assertion `left matches right` failed
  left: `{left:?}`
 right: `{right}`"#
		),
	}
}
