//! Utilities.

mod make_id;
pub(crate) use make_id::make_id;

mod name_or_id;
pub use name_or_id::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier};

mod addr_ext;
pub use addr_ext::AddrExt;

pub mod time;
pub mod num;
pub mod serde;
