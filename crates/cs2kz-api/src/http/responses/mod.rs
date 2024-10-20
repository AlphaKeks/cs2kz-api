//! Various generic HTTP responses.

mod error;
pub use error::ErrorResponse;

mod no_content;
pub use no_content::NoContent;

mod not_found;
pub use not_found::NotFound;
