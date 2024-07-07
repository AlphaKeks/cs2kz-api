//! The CS2KZ "standard library".
//!
//! This crate contains a set of core types and functions related to CS2KZ.
//! It is primarly used by the API, but may be published and used in other
//! projects in the future.

mod identifier;

pub mod steam_id;

#[doc(inline)]
pub use steam_id::SteamID;

pub mod mode;

#[doc(inline)]
pub use mode::Mode;

pub mod style;

#[doc(inline)]
pub use style::Style;

pub mod tier;

#[doc(inline)]
pub use tier::Tier;

pub mod jump_type;

#[doc(inline)]
pub use jump_type::JumpType;

mod player_identifier;

#[doc(inline)]
pub use player_identifier::PlayerIdentifier;

mod map_identifier;

#[doc(inline)]
pub use map_identifier::MapIdentifier;

mod course_identifier;

#[doc(inline)]
pub use course_identifier::CourseIdentifier;

mod server_identifier;

#[doc(inline)]
pub use server_identifier::ServerIdentifier;

mod global_status;

#[doc(inline)]
pub use global_status::GlobalStatus;

mod ranked_status;

#[doc(inline)]
pub use ranked_status::RankedStatus;
