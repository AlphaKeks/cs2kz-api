use std::future::Future;
use std::{cmp, fmt};

use axum::response::{IntoResponse, Response};
use futures_util::{TryStream, TryStreamExt as _};

use crate::http::extract::Json;

#[derive(Debug, Clone, Copy, serde::Serialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(transparent)]
pub struct Limit<const MAX: u64, const DEFAULT: u64>(u64);

impl<const MAX: u64, const DEFAULT: u64> Limit<MAX, DEFAULT> {
	/// Creates a new [`Limit`] object if `value` is less than or equal to
	/// `MAX`.
	pub fn new(value: u64) -> Option<Self> {
		if value <= MAX {
			Some(Self(value))
		} else {
			None
		}
	}

	/// Gets the inner value.
	pub fn get(&self) -> u64 {
		self.0
	}

	/// Returns `MAX`.
	///
	/// This is useful in contexts where the const generics are hard-coded.
	#[expect(clippy::unused_self)]
	pub fn max(&self) -> u64 {
		MAX
	}
}

impl<const MAX: u64, const DEFAULT: u64> Default for Limit<MAX, DEFAULT> {
	fn default() -> Self {
		Self(DEFAULT)
	}
}

impl<'de, const MAX: u64, const DEFAULT: u64> serde::Deserialize<'de> for Limit<MAX, DEFAULT> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		struct LimitVisitor<const MAX: u64, const DEFAULT: u64>;

		impl<'de, const MAX: u64, const DEFAULT: u64> de::Visitor<'de> for LimitVisitor<MAX, DEFAULT> {
			type Value = Limit<MAX, DEFAULT>;

			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
				write!(fmt, "an integer between 0 and {MAX}")
			}

			fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Limit::new(value)
					.ok_or_else(|| de::Error::invalid_value(de::Unexpected::Unsigned(value), &self))
			}

			fn visit_none<E>(self) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(Limit::default())
			}

			fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
			where
				D: serde::Deserializer<'de>,
			{
				deserializer.deserialize_u64(self)
			}
		}

		deserializer.deserialize_option(LimitVisitor::<MAX, DEFAULT>)
	}
}

#[derive(
	Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[sqlx(transparent)]
pub struct Offset(pub i64);

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PaginationResult<T> {
	/// How many results are available in total.
	pub total: u64,

	/// The values for this request.
	pub values: Vec<T>,
}

impl<T> PaginationResult<T> {
	#[expect(dead_code)] // TODO: remove?
	pub fn new(total: u64) -> Self {
		Self {
			total,
			values: Vec::new(),
		}
	}

	pub fn with_capacity(total: u64, capacity: usize) -> Self {
		Self {
			total,
			values: Vec::with_capacity(capacity),
		}
	}

	pub fn estimate_capacity(total: u64, max: u64) -> Self {
		cmp::min(total, max)
			.try_into()
			.map(|capacity| Self::with_capacity(total, capacity))
			.expect("64-bit platform")
	}
}

impl<T> IntoResponse for PaginationResult<T>
where
	T: serde::Serialize,
{
	fn into_response(self) -> Response {
		Json(self).into_response()
	}
}

pub trait TryStreamExt: TryStream<Ok: Send, Error: Send> + Send + Unpin {
	fn try_collect_into_pagination_result(
		self,
		total: u64,
		limit: u64,
	) -> impl Future<Output = Result<PaginationResult<Self::Ok>, Self::Error>> + Send;
}

impl<S> TryStreamExt for S
where
	S: TryStream<Ok: Send, Error: Send> + Send + Unpin,
{
	async fn try_collect_into_pagination_result(
		mut self,
		total: u64,
		limit: u64,
	) -> Result<PaginationResult<Self::Ok>, Self::Error> {
		let mut result = PaginationResult::estimate_capacity(total, limit);

		while let Some(value) = self.try_next().await? {
			result.values.push(value);
		}

		Ok(result)
	}
}
