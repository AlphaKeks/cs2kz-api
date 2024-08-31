//! Utilities for unit & integration tests.

mod macros;

#[allow(unused_imports)] // these may be used later
pub(crate) use macros::*;

pub type Error = anyhow::Error;
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[ctor::ctor]
fn ctor()
{
	// TODO
}
