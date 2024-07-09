//! The CS2KZ "standard library".
//!
//! This crate contains a set of core types and functions related to CS2KZ.
//! It is primarly used by the API, but may be published and used in other
//! projects in the future.

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

mod global_status;

#[doc(inline)]
pub use global_status::GlobalStatus;

mod ranked_status;

#[doc(inline)]
pub use ranked_status::RankedStatus;
