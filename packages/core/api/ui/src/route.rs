use hyper::{Body, Method, Request, Response};
use rivet_operation::prelude::*;

pub struct Router;

impl Router {
	#[doc(hidden)]
	#[tracing::instrument(skip_all)]
	pub async fn __inner(
		_shared_client: chirp_client::SharedClientHandle,
		config: rivet_config::Config,
		_pools: rivet_pools::Pools,
		_cache: rivet_cache::Cache,
		_ray_id: uuid::Uuid,
		request: &mut Request<Body>,
		response: &mut http::response::Builder,
		_router_config: &mut api_helper::macro_util::__RouterConfig,
	) -> rivet_operation::prelude::GlobalResult<Option<Vec<u8>>> {
		// Don't do anything if UI is not enabled.
		if !config.server()?.rivet.ui.enable() {
			return Ok(None);
		}

		let path = request.uri().path();

		// Redirect to UI
		if path == "/" || path == "/index.html" {
			tracing::debug!(path = ?path, "redirecting to /ui/");
			*response = std::mem::take(response)
				.status(hyper::StatusCode::TEMPORARY_REDIRECT)
				.header(hyper::header::LOCATION, "/ui/");
			return Ok(Some("Redirecting to /ui/".into()));
		}

		// Check if the path starts with "/ui"
		if !request.uri().path().starts_with("/ui") {
			return Ok(None);
		}

		// Build proxy URL by joining request path to base URL
		let mut proxy_url = config.server()?.rivet.ui.proxy_origin().clone();
		
		// Remove leading slash from request path since join() expects relative paths
		let request_path = request.uri().path().strip_prefix('/').unwrap_or(request.uri().path());
		
		// Join the request path to the base URL
		proxy_url = match proxy_url.join(request_path) {
			Ok(url) => url,
			Err(e) => bail!("Failed to build proxy URL: {}", e),
		};
		
		// Set query string if present
		proxy_url.set_query(request.uri().query());
		
		let full_proxy_url = proxy_url.to_string();

		tracing::debug!(
			original_path = ?path,
			proxy_url = ?full_proxy_url,
			"proxying request"
		);

		// Build reqwest request
		let client = reqwest::Client::new();
		let method = match reqwest::Method::from_bytes(request.method().as_str().as_bytes()) {
			Ok(method) => method,
			Err(e) => bail!("Invalid HTTP method: {}", e),
		};
		let mut req_builder = client.request(method, &full_proxy_url);

		// Forward headers
		for (name, value) in request.headers() {
			if let Ok(value_str) = value.to_str() {
				req_builder = req_builder.header(name.as_str(), value_str);
			}
		}

		// Forward body for non-GET requests
		if request.method() != Method::GET && request.method() != Method::HEAD {
			let body_bytes = match hyper::body::to_bytes(std::mem::replace(request.body_mut(), Body::empty())).await {
				Ok(bytes) => bytes,
				Err(e) => bail!("Failed to read request body: {}", e),
			};
			req_builder = req_builder.body(body_bytes.to_vec());
		}

		// Make the request
		let proxy_response = match req_builder.send().await {
			Ok(response) => response,
			Err(e) => bail!("Proxy request failed: {}", e),
		};

		// Set response status
		let status_code = proxy_response.status();
		let hyper_status = match hyper::StatusCode::from_u16(status_code.as_u16()) {
			Ok(status) => status,
			Err(e) => bail!("Invalid status code: {}", e),
		};
		*response = std::mem::take(response).status(hyper_status);

		// Forward response headers
		if let Some(headers) = response.headers_mut() {
			for (name, value) in proxy_response.headers() {
				if let Ok(header_name) =
					hyper::header::HeaderName::from_bytes(name.as_str().as_bytes())
				{
					if let Ok(header_value) =
						hyper::header::HeaderValue::from_bytes(value.as_bytes())
					{
						headers.insert(header_name, header_value);
					}
				}
			}
		}

		// Get response body
		let body_bytes = match proxy_response.bytes().await {
			Ok(bytes) => bytes,
			Err(e) => bail!("Failed to read proxy response body: {}", e),
		};

		Ok(Some(body_bytes.to_vec()))
	}

	#[tracing::instrument(skip_all)]
	pub async fn handle(
		shared_client: chirp_client::SharedClientHandle,
		config: rivet_config::Config,
		pools: rivet_pools::Pools,
		cache: rivet_cache::Cache,
		ray_id: uuid::Uuid,
		mut request: Request<Body>,
		mut response: http::response::Builder,
	) -> Result<Response<Body>, http::Error> {
		tracing::debug!(
			method = ?request.method(),
			uri = ?request.uri(),
			"received request"
		);

		let mut router_config =
			match api_helper::macro_util::__RouterConfig::new(&config, request.uri()) {
				Ok(x) => x,
				Err(err) => {
					return api_helper::error::handle_rejection(&config, err, response, ray_id)
				}
			};

		let res = Self::__inner(
			shared_client,
			config.clone(),
			pools,
			cache,
			ray_id,
			&mut request,
			&mut response,
			&mut router_config,
		)
		.await;

		match res {
			Ok(Some(content)) => response.body(Body::from(content)),
			Ok(None) => response.body(Body::empty()),
			Err(err) => api_helper::error::handle_rejection(&config, err, response, ray_id),
		}
	}
}
