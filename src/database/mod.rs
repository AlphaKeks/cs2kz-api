//! Database utilities.

mod error;

#[doc(inline)]
pub use error::SqlxErrorExt;

pub mod query;

#[doc(inline)]
pub use query::{FilteredQuery, QueryBuilderExt, UpdateQuery};

mod resolve_id;

#[doc(inline)]
pub use resolve_id::ResolveID;
