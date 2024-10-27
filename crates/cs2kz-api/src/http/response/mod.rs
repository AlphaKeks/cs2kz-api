//! Generic HTTP responses.

mod error;
pub use error::ErrorResponse;

mod created;
pub use created::Created;

mod no_content;
pub use no_content::NoContent;

mod not_found;
pub use not_found::NotFound;
