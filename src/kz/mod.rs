//! Extensions to the [`cs2kz`] crate.

mod style_flags;
pub use style_flags::StyleFlags;

mod name_or_id;
pub use name_or_id::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier};
