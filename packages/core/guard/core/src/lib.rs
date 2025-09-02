pub mod analytics;
pub mod cert_resolver;
pub mod custom_serve;
pub mod errors;
pub mod metrics;
pub mod proxy_service;
pub mod request_context;
mod server;
pub mod types;

pub use cert_resolver::CertResolverFn;
pub use custom_serve::CustomServeTrait;
pub use proxy_service::{
	CacheKeyFn, MiddlewareFn, ProxyService, ProxyState, RouteTarget, RoutingFn, RoutingOutput,
};

// Re-export hyper StatusCode for use in other crates
pub mod status {
	pub use hyper::StatusCode;
}
pub use server::run_server;
pub use types::{EndpointType, GameGuardProtocol};
