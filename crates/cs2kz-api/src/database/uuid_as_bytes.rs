macro_rules! uuid_as_bytes {
	($struct:ty) => {
		impl<DB> sqlx::Type<DB> for $struct
		where
			DB: sqlx::Database,
			[u8]: sqlx::Type<DB>,
		{
			fn type_info() -> <DB as sqlx::Database>::TypeInfo {
				<[u8] as sqlx::Type<DB>>::type_info()
			}

			fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
				<[u8] as sqlx::Type<DB>>::compatible(ty)
			}
		}

		impl<'q, DB> sqlx::Encode<'q, DB> for $struct
		where
			DB: sqlx::Database,
			for<'a> &'a [u8]: sqlx::Encode<'q, DB>,
		{
			fn encode_by_ref(
				&self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				<&[u8] as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0.as_bytes().as_slice(), buf)
			}

			fn encode(
				self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
				<&[u8] as sqlx::Encode<'q, DB>>::encode(self.0.as_bytes(), buf)
			}

			fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
				<&[u8] as sqlx::Encode<'q, DB>>::produces(&self.0.as_bytes().as_slice())
			}

			fn size_hint(&self) -> usize {
				<&[u8] as sqlx::Encode<'q, DB>>::size_hint(&self.0.as_bytes().as_slice())
			}
		}

		impl<'r, DB> sqlx::Decode<'r, DB> for $struct
		where
			DB: sqlx::Database,
			&'r [u8]: sqlx::Decode<'r, DB>,
		{
			fn decode(
				value: <DB as sqlx::Database>::ValueRef<'r>,
			) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
				<&'r [u8] as sqlx::Decode<'r, DB>>::decode(value)
					.map(Uuid::from_slice)?
					.map(Self)
					.map_err(Into::into)
			}
		}
	};
}

pub(crate) use uuid_as_bytes;
