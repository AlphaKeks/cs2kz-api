#[allow(clippy::disallowed_types)]
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use axum::extract::ConnectInfo;
use tower_http::validate_request::ValidateRequest;

#[derive(Debug, Clone, Copy)]
pub struct IsLocalhost;

impl ValidateRequest<crate::http::Body> for IsLocalhost
{
	type ResponseBody = crate::http::Body;

	#[allow(clippy::disallowed_types)]
	fn validate(
		&mut self,
		request: &mut crate::http::Request,
	) -> Result<(), http::Response<Self::ResponseBody>>
	{
		request
			.extensions()
			.get::<ConnectInfo<SocketAddr>>()
			.and_then(|&ConnectInfo(addr)| match addr {
				SocketAddr::V4(addr) if addr.ip() == &Ipv4Addr::LOCALHOST => Some(()),
				SocketAddr::V6(addr) if addr.ip() == &Ipv6Addr::LOCALHOST => Some(()),
				_ => None,
			})
			.ok_or_else(Default::default)
	}
}
