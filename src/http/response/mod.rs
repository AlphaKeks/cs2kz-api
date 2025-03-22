//! Custom [response] types
//!
//! [response]: axum::response::IntoResponse

mod created;
mod error;

pub(crate) use self::{
	created::Created,
	error::{HandlerError, HandlerResult},
};
