pub mod auth;
pub mod errors;
mod proxy_state;
mod request_context;
mod response_body;

pub use proxy_state::{ProxyState, ProxyStateInner};
pub use request_context::RequestContext;
pub use response_body::{ResponseBody, ResponseBodyError, ResponseBodyTerminal};

/// HTTP header that carries the ray id guard assigned to the request, on every response.
pub const X_RIVET_RAY_ID: &str = "x-rivet-ray-id";
