pub mod problem;
pub use problem::Problem;

pub type Body = axum::body::Body;
pub type Request = axum::extract::Request;
pub type Response = axum::response::Response;

pub type ProblemDetails = problem_details::ProblemDetails<Problem>;
