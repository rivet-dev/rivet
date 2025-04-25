pub mod cert_resolver;
pub mod metrics;
pub mod proxy_service;
mod server;
pub mod types;
pub mod util;

pub use cert_resolver::CertResolverFn;
pub use proxy_service::{MiddlewareFn, ProxyService, ProxyState, RouteTarget, RoutingFn};

// Re-export hyper StatusCode for use in other crates
pub mod status {
	pub use hyper::StatusCode;
}
pub use server::{run_server, GlobalErrorWrapper};
pub use types::{EndpointType, GameGuardProtocol};
