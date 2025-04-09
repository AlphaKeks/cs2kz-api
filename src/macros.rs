/// Implements the [`Distribution`] trait for [`StandardUniform`] over a given
/// type.
///
/// # Examples
///
/// ```ignore
/// struct MyType(i32);
///
/// impl_rand!(MyType => |rng| MyType(rng.random::<i32>()));
/// ```
///
/// [`Distribution`]: ::rand::distr::Distribution
/// [`StandardUniform`]: ::rand::distr::StandardUniform
macro_rules! impl_rand {
	($type:ty => |$rng:pat_param| $impl:expr) => {
		#[cfg(feature = "rand")]
		impl ::rand::distr::Distribution<$type> for ::rand::distr::StandardUniform
		{
			fn sample<R: ::rand::Rng + ?Sized>(&self, $rng: &mut R) -> $type
			{
				$impl
			}
		}
	};
}

/// Implements the [`sqlx::Type`], [`sqlx::Encode`], and [`sqlx::Decode`] traits
/// for a given type.
///
/// # Examples
///
/// ```ignore
/// struct MyType([u8; 16]);
///
/// impl_sqlx!(MyType => {
///     Type as [u8];
///
///     //     vv lifetime for the trait impl
///     //         vv higher-ranked lifetime bound
///     Encode<'q, 'a> as &'a [u8] = |me| &me.0[..];
///
///     //     vv lifetime for the trait impl
///     Decode<'r> as &'r [u8] = |bytes| <[u8; 16]>::try_from(bytes).map(Self);
/// });
/// ```
macro_rules! impl_sqlx {
	($type:ty => {
		Type as $type_ty:ty;
		Encode<$encode_lt:lifetime $(,$encode_hrlt:lifetime)* $(,)?> as $encode_ty:ty = |$encode_self:ident| $encode_impl:expr;
		Decode<$decode_lt:lifetime $(,$decode_hrlt:lifetime)* $(,)?> as $decode_ty:ty = |$decode_value:pat_param| $decode_impl:expr;
	}) => {
		impl<DB> ::sqlx::Type<DB> for $type
		where
			DB: ::sqlx::Database,
			$type_ty: ::sqlx::Type<DB>,
		{
			fn type_info() -> <DB as ::sqlx::Database>::TypeInfo
			{
				<$type_ty as ::sqlx::Type<DB>>::type_info()
			}

			fn compatible(ty: &<DB as ::sqlx::Database>::TypeInfo) -> ::std::primitive::bool
			{
				<$type_ty as ::sqlx::Type<DB>>::compatible(ty)
			}
		}

		impl<$encode_lt, DB> ::sqlx::Encode<$encode_lt, DB> for $type
		where
			DB: ::sqlx::Database,
			for<$($encode_hrlt),*> $encode_ty: ::sqlx::Encode<$encode_lt, DB>,
		{
			#[::tracing::instrument(level = "trace", skip(buf), err)]
			fn encode(
				self,
				buf: &mut <DB as ::sqlx::Database>::ArgumentBuffer<$encode_lt>,
			) -> Result<
				::sqlx::encode::IsNull,
				::std::boxed::Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync>,
			>
			{
				let $encode_self = self;
				::sqlx::Encode::encode({ $encode_impl }, buf)
			}

			#[::tracing::instrument(level = "trace", skip(buf), err)]
			fn encode_by_ref(
				&self,
				buf: &mut <DB as ::sqlx::Database>::ArgumentBuffer<$encode_lt>,
			) -> Result<
				::sqlx::encode::IsNull,
				::std::boxed::Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync>,
			>
			{
				let $encode_self = self;
				::sqlx::Encode::encode(&{ $encode_impl }, buf)
			}

			fn produces(&self) -> Option<<DB as ::sqlx::Database>::TypeInfo>
			{
				let $encode_self = self;
				::sqlx::Encode::produces(&{ $encode_impl })
			}

			fn size_hint(&self) -> ::std::primitive::usize
			{
				let $encode_self = self;
				::sqlx::Encode::size_hint(&{ $encode_impl })
			}
		}

		impl<$decode_lt, DB> ::sqlx::Decode<$decode_lt, DB> for $type
		where
			DB: ::sqlx::Database,
			for<$($decode_hrlt),*> $decode_ty: ::sqlx::Decode<$decode_lt, DB>,
		{
			#[::tracing::instrument(
				level = "trace",
				skip(value),
				ret(level = "trace"),
				err(level = "debug"),
			)]
			fn decode(
				value: <DB as sqlx::Database>::ValueRef<$decode_lt>,
			) -> Result<
				Self,
				::std::boxed::Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync>,
			>
			{
				match ::sqlx::Decode::decode(value) {
					::std::result::Result::Ok($decode_value) => ::std::result::Result::map_err($decode_impl, ::std::convert::Into::into),
					::std::result::Result::Err(err) => ::std::result::Result::Err(err),
				}
			}
		}
	};
}

/// Implements the [`utoipa::ToSchema`], and [`utoipa::PartialSchema`] traits
/// for a given type.
///
/// # Examples
///
/// ```ignore
/// struct MyType([u8; 16]);
///
/// impl_utoipa!(MyType => {
///     Object::builder()
///         .description(Some("some bytes"))
///         .schema_type(schema::Type::String)
///         .min_length(Some(16))
///         .max_length(Some(16))
/// });
/// ```
macro_rules! impl_utoipa {
	($type:ty => $impl:expr) => {
		impl ::utoipa::ToSchema for $type
		{
		}

		impl ::utoipa::PartialSchema for $type
		{
			fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema>
			{
				#[allow(unused_imports)]
				use ::utoipa::openapi::{
					Object,
					schema::{self, SchemaType},
				};

				::std::convert::Into::into({ $impl })
			}
		}
	};
}
