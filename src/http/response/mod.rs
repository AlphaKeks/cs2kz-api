//! Custom [response] types
//!
//! [response]: axum::response::IntoResponse

pub(crate) use self::{
	created::Created,
	error::{HandlerError, HandlerResult},
};

mod created;
mod error;
