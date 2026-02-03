use anyhow::{Context, Result};
use hyper::{Method, header::HeaderMap};
use rivet_runner_protocol as protocol;
use rivet_util::Id;
use std::{
	net::{IpAddr, SocketAddr},
	time::Instant,
};

#[derive(Clone)]
pub struct RequestContext {
	pub(crate) remote_addr: SocketAddr,
	pub(crate) ray_id: Id,
	pub(crate) req_id: Id,
	/// Entire host including port (if present)
	pub(crate) host: String,
	/// Only hostname, no port.
	pub(crate) hostname: String,
	/// Includes path and query.
	pub(crate) path: String,
	pub(crate) method: Method,
	pub(crate) headers: HeaderMap,
	pub(crate) is_websocket: bool,
	pub(crate) client_ip: IpAddr,
	pub(crate) start_time: Instant,

	pub(crate) rate_limit: RateLimitConfig,
	pub(crate) max_in_flight: MaxInFlightConfig,
	pub(crate) retry: RetryConfig,
	pub(crate) timeout: TimeoutConfig,

	pub(crate) in_flight_request_id: Option<protocol::RequestId>,
	pub(crate) cors: Option<CorsConfig>,
}

impl RequestContext {
	pub(crate) fn new(
		remote_addr: SocketAddr,
		ray_id: Id,
		req_id: Id,
		host: String,
		path: String,
		method: Method,
		headers: HeaderMap,
		is_websocket: bool,
		client_ip: IpAddr,
		start_time: Instant,
	) -> Self {
		let hostname = host.split(':').next().unwrap_or(&host).to_string();

		RequestContext {
			remote_addr,
			ray_id,
			req_id,
			host,
			hostname,
			path,
			method,
			headers,
			is_websocket,
			client_ip,
			start_time,

			rate_limit: RateLimitConfig {
				requests: 10000, // 10000 requests
				period: 60,      // per 60 seconds
			},
			max_in_flight: MaxInFlightConfig {
				amount: 2000, // 2000 concurrent requests
			},
			retry: RetryConfig {
				max_attempts: 7,       // 7 retry attempts
				initial_interval: 150, // 150ms initial interval
			},
			timeout: TimeoutConfig {
				request_timeout: 30, // 30 seconds for requests
			},

			in_flight_request_id: None,
			cors: None,
		}
	}

	pub fn ray_id(&self) -> Id {
		self.ray_id
	}

	pub fn req_id(&self) -> Id {
		self.req_id
	}

	pub fn host(&self) -> &str {
		&self.host
	}

	pub fn hostname(&self) -> &str {
		&self.hostname
	}

	pub fn path(&self) -> &str {
		&self.path
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

	pub fn in_flight_request_id(&self) -> Result<protocol::RequestId> {
		self.in_flight_request_id
			.context("no in flight request id acquired")
	}

	pub fn set_cors(&mut self, cors_config: CorsConfig) {
		self.cors = Some(cors_config);
	}
}

#[derive(Clone, Debug)]
pub struct RateLimitConfig {
	pub requests: u64,
	pub period: u64, // in seconds
}

#[derive(Clone, Debug)]
pub struct MaxInFlightConfig {
	pub amount: usize,
}

#[derive(Clone, Debug)]
pub struct RetryConfig {
	pub max_attempts: u32,
	pub initial_interval: u64, // in milliseconds
}

#[derive(Clone, Debug)]
pub struct TimeoutConfig {
	pub request_timeout: u64, // in seconds
}

#[derive(Clone, Debug)]
pub struct CorsConfig {
	pub allow_origin: String,
	pub allow_credentials: bool,
	pub expose_headers: String,

	// Only set for OPTIONS requests
	// TODO: Vec of Method
	pub allow_methods: Option<String>,
	pub allow_headers: Option<String>,
	// Seconds
	pub max_age: Option<u32>,
}
