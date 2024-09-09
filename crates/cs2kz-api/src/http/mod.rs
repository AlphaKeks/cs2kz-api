#![allow(unused)]

pub mod problem;
pub use problem::Problem;

mod location;
pub use location::Location;

mod created;
pub use created::Created;

mod no_content;
pub use no_content::NoContent;

pub type Body = axum::body::Body;
pub type Request = axum::extract::Request;
pub type Response = axum::response::Response;

pub type ProblemDetails = problem_details::ProblemDetails<Problem>;
pub type Result<T> = std::result::Result<T, ProblemDetails>;
