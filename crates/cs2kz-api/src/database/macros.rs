/// Implements [`sqlx::Type`], [`sqlx::Encode`], and [`sqlx::Decode`] for wrapper types, in terms
/// of the wrapped type.
///
/// # Example
///
/// If the wrapper type is a standard new-type:
///
/// ```ignore
/// struct MyID(u64);
///
/// wrap!(MyID as u64);
/// ```
///
/// If some extra logic is required:
///
/// ```ignore
/// struct MyThing { ... }
///
/// wrap!(MyThing as Vec<u8> => {
///     get: |self /* : MyThing */| /* -> Vec<u8> */ { ... };
///     get_ref: |self /* : &MyThing */| /* -> &Vec<u8> */ { ... };
///     make: |value /* : Vec<u8> */| /* -> database::Result<MyThing> */ { ... };
/// });
/// ```
///
/// A special case is strings / byte slices: you write `as str` / `as [u8]` in the macro
/// invocation, but use `&str`s / `&[u8]`s in the closures.
macro_rules! wrap {
	($ty:ty as $wrapped:ty) => {
		$crate::database::macros::wrap!($ty as $wrapped => {
			get: |self| self.0;
			get_ref: |self| &self.0;
			make: |value| Ok(Self(value));
		});
	};
	($ty:ty as str => {
		get: |$self:ident| $get:expr;
		make: |$value:ident| $make:expr;
	}) => {
		$crate::database::macros::wrap!(@unsized $ty as str => {
			get: |$self| $get;
			make: |$value| $make;
		});
	};
	($ty:ty as [u8] => {
		get: |$self:ident| $get:expr;
		make: |$value:ident| $make:expr;
	}) => {
		$crate::database::macros::wrap!(@unsized $ty as [u8] => {
			get: |$self| $get;
			make: |$value| $make;
		});
	};
	($ty:ty as $wrapped:ty => {
		get: |$self:ident| $get:expr;
		get_ref: |$self_ref:ident| $get_ref:expr;
		make: |$value:ident| $make:expr;
	}) => {
		impl<DB> sqlx::Type<DB> for $ty
		where
			DB: sqlx::Database,
			$wrapped: sqlx::Type<DB>,
		{
			fn type_info() -> <DB as sqlx::Database>::TypeInfo {
				<$wrapped as sqlx::Type<DB>>::type_info()
			}

			fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
				<$wrapped as sqlx::Type<DB>>::compatible(ty)
			}
		}

		impl<'q, DB> sqlx::Encode<'q, DB> for $ty
		where
			DB: sqlx::Database,
			$wrapped: sqlx::Encode<'q, DB>,
		{
			fn encode_by_ref(
				&$self_ref,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				<$wrapped as sqlx::Encode<'q, DB>>::encode_by_ref($get_ref, buf)
			}

			fn encode(
				$self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				<$wrapped as sqlx::Encode<'q, DB>>::encode($get, buf)
			}

			fn produces(&$self_ref) -> Option<<DB as sqlx::Database>::TypeInfo> {
				<$wrapped as sqlx::Encode<'q, DB>>::produces($get_ref)
			}

			fn size_hint(&$self_ref) -> usize {
				<$wrapped as sqlx::Encode<'q, DB>>::size_hint($get_ref)
			}
		}

		impl<'r, DB> sqlx::Decode<'r, DB> for $ty
		where
			DB: sqlx::Database,
			$wrapped: sqlx::Decode<'r, DB>,
		{
			fn decode(
				value: <DB as sqlx::Database>::ValueRef<'r>,
			) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
				let $value = <$wrapped as sqlx::Decode<'r, DB>>::decode(value)?;
				$make
			}
		}
	};
	(@unsized $ty:ty as $wrapped:ty => {
		get: |$self:ident| $get:expr;
		make: |$value:ident| $make:expr;
	}) => {
		impl<DB> sqlx::Type<DB> for $ty
		where
			DB: sqlx::Database,
			$wrapped: sqlx::Type<DB>,
		{
			fn type_info() -> <DB as sqlx::Database>::TypeInfo {
				<$wrapped as sqlx::Type<DB>>::type_info()
			}

			fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
				<$wrapped as sqlx::Type<DB>>::compatible(ty)
			}
		}

		impl<'q, DB> sqlx::Encode<'q, DB> for $ty
		where
			DB: sqlx::Database,
			for<'a> &'a $wrapped: sqlx::Encode<'q, DB>,
		{
			fn encode_by_ref(
				&$self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				<&$wrapped as sqlx::Encode<'q, DB>>::encode_by_ref(&$get, buf)
			}

			fn encode(
				$self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				<&$wrapped as sqlx::Encode<'q, DB>>::encode($get, buf)
			}

			fn produces(&$self) -> Option<<DB as sqlx::Database>::TypeInfo> {
				<&$wrapped as sqlx::Encode<'q, DB>>::produces(&$get)
			}

			fn size_hint(&$self) -> usize {
				<&$wrapped as sqlx::Encode<'q, DB>>::size_hint(&$get)
			}
		}

		impl<'r, DB> sqlx::Decode<'r, DB> for $ty
		where
			DB: sqlx::Database,
			&'r $wrapped: sqlx::Decode<'r, DB>,
		{
			fn decode(
				value: <DB as sqlx::Database>::ValueRef<'r>,
			) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
				let $value = <&'r $wrapped as sqlx::Decode<'r, DB>>::decode(value)?;
				$make
			}
		}
	};
}

pub(crate) use wrap;
