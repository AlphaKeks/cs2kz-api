//! Testing helpers and a [`ctor`] that performs setup before any tests.

use std::env;
use std::panic::Location;

use url::Url;

pub type Result<T = ()> = std::result::Result<T, Error>;

/// Error type to use for tests.
///
/// This can be created from any other error type and stores the source location at which it was
/// created. This makes it a lot easier to figure out where exactly a test failed.
#[derive(Debug)]
#[debug("{} ({})", source, location)]
pub struct Error {
	source: Box<dyn std::error::Error>,
	location: &'static Location<'static>,
}

impl<E> From<E> for Error
where
	E: Into<Box<dyn std::error::Error>>,
{
	#[track_caller]
	fn from(error: E) -> Self {
		Self {
			source: error.into(),
			location: Location::caller(),
		}
	}
}

/// Setup function that runs before any tests.
///
/// See the [`ctor`] documentation for more information.
#[ctor::ctor]
fn test_setup() {
	tracing_subscriber::fmt::init();

	if let Ok(database_url) = env::var("DATABASE_URL") {
		let mut database_url = database_url.parse::<Url>().unwrap();
		database_url.set_username("root").unwrap();
		env::set_var("DATABASE_URL", database_url.as_str());
	}
}
