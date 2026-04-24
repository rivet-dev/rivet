use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use rivet_guard_core::{
	ResponseBody,
	custom_serve::CustomServeTrait,
	request_context::{CorsConfig, RequestContext},
};

pub fn origin_header(req_ctx: &RequestContext) -> String {
	req_ctx
		.headers()
		.get("origin")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("*")
		.to_string()
}

pub fn set_non_preflight_cors(req_ctx: &mut RequestContext) {
	let allow_origin = origin_header(req_ctx);
	req_ctx.set_cors(CorsConfig {
		allow_origin,
		allow_credentials: true,
		expose_headers: "*".to_string(),
		allow_methods: None,
		allow_headers: None,
		max_age: None,
	});
}

/// Responds to CORS preflight OPTIONS requests with 204 and permissive CORS
/// headers. Avoids actor lookup, wake, and auth because browsers cannot attach
/// credentials to preflights. The actual request that follows is still authed.
pub struct CorsPreflight;

#[async_trait]
impl CustomServeTrait for CorsPreflight {
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		let allow_origin = req
			.headers()
			.get("origin")
			.and_then(|v| v.to_str().ok())
			.unwrap_or("*")
			.to_string();
		let allow_headers = req
			.headers()
			.get("access-control-request-headers")
			.and_then(|v| v.to_str().ok())
			.unwrap_or("*")
			.to_string();

		req_ctx.set_cors(CorsConfig {
			allow_origin,
			allow_credentials: true,
			expose_headers: "*".to_string(),
			allow_methods: Some("GET, POST, PUT, DELETE, OPTIONS, PATCH".to_string()),
			allow_headers: Some(allow_headers),
			max_age: Some(86400),
		});

		Ok(Response::builder()
			.status(StatusCode::NO_CONTENT)
			.body(ResponseBody::Full(Full::new(Bytes::new())))?)
	}
}
