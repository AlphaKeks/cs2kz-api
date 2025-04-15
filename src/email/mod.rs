pub use self::{
	address::{EmailAddress, ParseEmailAddressError},
	client::Client,
};

mod address;
pub mod client;
