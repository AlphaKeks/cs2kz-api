/// Generates a transparent integer wrapper, useful for ID types.
///
/// # Example
///
/// ```ignore
/// make_id! {
///     /// Some documentation.
///     pub struct MyID(u16);
/// }
/// ```
macro_rules! make_id {
	(
		$(#[$meta:meta])*
		$vis:vis struct $name:ident($inner:ty);
	) => {
		$(#[$meta])*
		#[repr(transparent)]
		#[derive(
			::std::fmt::Debug,
			::std::clone::Clone,
			::std::marker::Copy,
			::std::cmp::PartialEq,
			::std::cmp::Eq,
			::std::cmp::PartialOrd,
			::std::cmp::Ord,
			::std::hash::Hash,
			::serde::Serialize,
			::serde::Deserialize,
			::sqlx::Type,
		)]
		#[serde(transparent)]
		$vis struct $name(pub ::std::num::NonZero<$inner>);

		impl $name
		{
			pub const fn value(&self) -> $inner
			{
				self.0.get()
			}
		}

		impl ::std::fmt::Display for $name
		{
			fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result
			{
				::std::fmt::Display::fmt(&self.0, f)
			}
		}

		impl ::std::convert::From<::std::num::NonZero<$inner>> for $name
		{
			fn from(value: ::std::num::NonZero<$inner>) -> Self
			{
				Self(value)
			}
		}

		impl ::std::convert::From<$name> for ::std::num::NonZero<$inner>
		{
			fn from($name(value): $name) -> Self
			{
				value
			}
		}

		impl ::std::convert::From<$name> for $inner
		{
			fn from(id: $name) -> Self
			{
				id.value()
			}
		}
	};
}

/// Implements the relevant [`sqlx`] traits for a given type.
///
/// # Example
///
/// ```ignore
/// sql_type!(MyType as SomeOtherType => {
///     encode_by_ref: |self| { /* … */ },
///     encode: |self| { /* … */ },
///     decode: |value| { /* … */ },
/// });
/// ```
macro_rules! sql_type {
	($type:ty as $repr:ty => {
		encode_by_ref: |$self_ref:ident| $encode_by_ref:expr,
		encode: |$self:ident| $encode:expr,
		decode: |$value:ident| $decode:expr,
	}) => {
		impl<DB> ::sqlx::Type<DB> for $type
		where
			DB: ::sqlx::Database,
			$repr: ::sqlx::Type<DB>,
		{
			fn type_info() -> <DB as ::sqlx::Database>::TypeInfo
			{
				<$repr as ::sqlx::Type<DB>>::type_info()
			}

			fn compatible(ty: &<DB as ::sqlx::Database>::TypeInfo) -> ::std::primitive::bool
			{
				<$repr as ::sqlx::Type<DB>>::compatible(ty)
			}
		}

		impl<'q, DB> ::sqlx::Encode<'q, DB> for $type
		where
			DB: ::sqlx::Database,
			$repr: ::sqlx::Encode<'q, DB>,
		{
			fn encode_by_ref(
				&$self_ref,
				buf: &mut <DB as ::sqlx::Database>::ArgumentBuffer<'q>,
			) -> ::std::result::Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError>
			{
				<$repr as ::sqlx::Encode<'q, DB>>::encode_by_ref($encode_by_ref, buf)
			}

			fn encode(
				$self,
				buf: &mut <DB as ::sqlx::Database>::ArgumentBuffer<'q>,
			) -> ::std::result::Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError>
			where
				Self: Sized,
			{
				<$repr as ::sqlx::Encode<'q, DB>>::encode({ $encode }, buf)
			}

			fn produces(&$self_ref) -> ::std::option::Option<<DB as ::sqlx::Database>::TypeInfo>
			{
				<$repr as ::sqlx::Encode<'q, DB>>::produces({ $encode_by_ref })
			}

			fn size_hint(&$self_ref) -> ::std::primitive::usize
			{
				<$repr as ::sqlx::Encode<'q, DB>>::size_hint({ $encode_by_ref })
			}
		}

		impl<'r, DB> ::sqlx::Decode<'r, DB> for $type
		where
			DB: ::sqlx::Database,
			$repr: ::sqlx::Decode<'r, DB>,
		{
			fn decode(
				value: <DB as ::sqlx::Database>::ValueRef<'r>,
			) -> ::std::result::Result<Self, ::sqlx::error::BoxDynError>
			{
				let $value = <$repr as ::sqlx::Decode<'r, DB>>::decode(value)?;

				{
					$decode
				}
			}
		}
	};
}

/// Defines a type that implements [`problem_details::ProblemType`].
macro_rules! problem_type {
	(
		$(#[$meta:meta])*
		$vis:vis enum $problem_type:ident
		{
			$(
				$(#[cfg($cfg:ident)])?
				#[title = $title:literal]
				#[status = $status:ident]
				$(#[$problem_meta:meta])*
				$problem:ident = $fragment:literal
			),*
			$(,)?
		}

		$(#[$error_meta:meta])*
		$error_vis:vis struct $error:ident;
	) => {
		$(#[$meta])*
		$vis enum $problem_type
		{
			$(
				$(#[cfg($cfg)])?
				$(#[$problem_meta])*
				#[error($title)]
				$problem
			),*
		}

		$(#[$error_meta])*
		$error_vis struct $error;

		impl ::problem_details::ProblemType for $problem_type
		{
			type ParseFragmentError = $error;

			fn base_uri() -> ::http::Uri
			{
				BASE_URI
					.get()
					.cloned()
					.expect("problem details base uri has not been initialized")
			}

			fn parse_fragment(fragment: &str) -> ::std::result::Result<Self, Self::ParseFragmentError>
			{
				match fragment {
					$( $(#[cfg($cfg)])? $fragment => Ok(Self::$problem), )*
					_ => Err($error),
				}
			}

			fn fragment(&self) -> &::std::primitive::str
			{
				match *self {
					$( $(#[cfg($cfg)])? Self::$problem => $fragment, )*
				}
			}

			fn status(&self) -> ::http::StatusCode
			{
				match *self {
					$( $(#[cfg($cfg)])? Self::$problem => ::http::StatusCode::$status, )*
				}
			}

			fn title(&self) -> &::std::primitive::str
			{
				match *self {
					$( $(#[cfg($cfg)])? Self::$problem => $title, )*
				}
			}
		}
	};
}

/// Implements [`axum::response::IntoResponse`] for a type that already implements
/// [`problem_details::AsProblemDetails`].
macro_rules! impl_into_response {
	($type:ty) => {
		impl ::axum::response::IntoResponse for $type
		{
			fn into_response(self) -> ::axum::response::Response
			{
				::axum::response::IntoResponse::into_response(
					::problem_details::AsProblemDetails::as_problem_details(&self),
				)
			}
		}
	};
}
