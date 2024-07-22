//! Helper traits & types for SQL.

mod error;
pub use error::SqlErrorExt;

mod query;
pub use query::{FilteredQueryBuilder, QueryBuilderExt, TransactionExt, UpdateQueryBuilder};

/// The minimum number of database pool connections.
pub const MIN_CONNECTIONS: u32 = match (cfg!(test), cfg!(feature = "production")) {
	(true, _) => 1,
	(false, false) => 20,
	(false, true) => 200,
};

/// The maximum number of database pool connections.
pub const MAX_CONNECTIONS: u32 = match (cfg!(test), cfg!(feature = "production")) {
	(true, _) => 10,
	(false, false) => 50,
	(false, true) => 256,
};
