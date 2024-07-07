//! The CS2KZ "standard library".
//!
//! This crate contains a set of core types and functions related to CS2KZ.
//! It is primarly used by the API, but may be published and used in other
//! projects in the future.

mod steam_id;

#[doc(inline)]
pub use steam_id::SteamID;
