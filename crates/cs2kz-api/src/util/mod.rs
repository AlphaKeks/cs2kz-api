//! Internal utilities.

pub mod net;
pub mod num;
pub mod serde;
pub mod time;

mod git_revision;
pub use git_revision::GitRevision;

mod non_empty;
pub use non_empty::NonEmpty;
