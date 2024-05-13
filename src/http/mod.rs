//! General HTTP / handler utilities.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod error;

#[doc(inline)]
pub use error::{HandlerError, HandlerResult};

pub mod middleware;
pub mod cors;

/// General purpose response body for pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Pagination<T>
where
	T: ToSchema<'static>,
{
	/// The total amount of available results.
	total: u64,

	/// The results for this request.
	#[schema(inline)]
	results: Vec<T>,
}

impl<T> Pagination<T>
where
	T: ToSchema<'static>,
{
	/// Creates a new [`Pagination<T>`] object.
	pub fn new(total: u64, results: Vec<T>) -> Self {
		Self { total, results }
	}
}
