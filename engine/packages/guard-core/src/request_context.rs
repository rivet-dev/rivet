use axum::extract::{Extension, FromRequestParts, rejection::ExtensionRejection};
use hyper::{Method, Uri, header::HeaderMap, http::request::Parts};
use rivet_util::Id;
use std::{
	net::{IpAddr, SocketAddr},
	time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

use crate::proxy_state::ProxyState;

#[derive(Clone)]
pub struct RequestContext {
	pub(crate) ray_id: Id,
	pub(crate) uri: Uri,
	pub(crate) method: Method,
	pub(crate) headers: HeaderMap,
	pub(crate) is_websocket: bool,
	pub(crate) start_time: Instant,
	pub(crate) client_disconnect: CancellationToken,
}

impl RequestContext {
	pub fn new(
		_proxy_state: ProxyState,
		_remote_addr: SocketAddr,
		ray_id: Id,
		uri: Uri,
		method: Method,
		headers: HeaderMap,
		is_websocket: bool,
		_client_ip: IpAddr,
		start_time: Instant,
		client_disconnect: CancellationToken,
	) -> Self {
		RequestContext {
			ray_id,
			uri,
			method,
			headers,
			is_websocket,
			start_time,
			client_disconnect,
		}
	}

	pub fn ray_id(&self) -> Id {
		self.ray_id
	}

	pub fn uri(&self) -> &Uri {
		&self.uri
	}

	pub fn method(&self) -> &Method {
		&self.method
	}

	pub fn headers(&self) -> &HeaderMap {
		&self.headers
	}

	pub fn is_websocket(&self) -> bool {
		self.is_websocket
	}

	/// NOTE: Only use this method for metrics. It does not use simulated time.
	pub fn elapsed(&self) -> Duration {
		self.start_time.elapsed()
	}

	pub fn client_disconnect_token(&self) -> CancellationToken {
		self.client_disconnect.clone()
	}
}

/// Reads the `RequestContext` that guard inserts into the request extensions before routing.
impl<S> FromRequestParts<S> for RequestContext
where
	S: Send + Sync,
{
	type Rejection = ExtensionRejection;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let Extension(req_ctx) =
			Extension::<RequestContext>::from_request_parts(parts, state).await?;

		Ok(req_ctx)
	}
}
