use hyper::{Body, Request, Response};
use rivet_operation::prelude::*;

pub struct Router;

impl Router {
	fn replace_vite_app_api_url(
		content: &[u8],
		config: &rivet_config::Config,
	) -> GlobalResult<Vec<u8>> {
		let content_str = std::str::from_utf8(content)?;

		let replacement_count = content_str.matches("%VITE_APP_API_URL%").count();
		ensure!(
			replacement_count == 1,
			"Expected exactly one occurrence of %VITE_APP_API_URL%, found {}",
			replacement_count
		);

		let public_origin = config.server()?.rivet.api_public.public_origin().to_string();
		let replaced_content = content_str.replace("%VITE_APP_API_URL%", &public_origin);

		Ok(replaced_content.into_bytes())
	}

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
			tracing::info!(path = ?path, "redirecting to /ui/");
			*response = std::mem::take(response)
				.status(hyper::StatusCode::TEMPORARY_REDIRECT)
				.header(hyper::header::LOCATION, "/ui/");
			return Ok(Some("Redirecting to /ui/".into()));
		}

		// Check if the path starts with "/ui"
		if !request.uri().path().starts_with("/ui") {
			return Ok(None);
		}

		// Strip the prefix to:
		// - Strip the mount path of /ui.
		// - Strip the starting slash in order to match the format `include_dir` needs.
		let path = request.uri().path().trim_start_matches("/ui/");
		let content = rivet_hub_embed::get_file_content(path);

		match content {
			Some(content) => {
				let content_type = mime_guess::from_path(&path).first_or_octet_stream();
				tracing::info!(
					path = ?path,
					?content_type,
					length = ?content.len(),
					"serving file"
				);
				if let Some(headers) = response.headers_mut() {
					headers.insert(
						hyper::header::CONTENT_TYPE,
						hyper::header::HeaderValue::from_str(content_type.as_ref()).unwrap(),
					);
				}

				// Replace VITE_APP_API_URL if the file is index.html
				let content = if path == "index.html" {
					Self::replace_vite_app_api_url(content, &config)?
				} else {
					content.to_vec()
				};

				Ok(Some(content))
			}
			None => {
				if path.ends_with(".html") || !path.contains('.') {
					tracing::info!(
						path = ?path,
						"file not found, serving index.html"
					);

					// Serve index.html content
					let index_content = unwrap!(
						rivet_hub_embed::get_file_content("index.html"),
						"index.html not found"
					);

					// Replace VITE_APP_API_URL in index.html
					let index_content = Self::replace_vite_app_api_url(index_content, &config)?;

					if let Some(headers) = response.headers_mut() {
						headers.insert(
							hyper::header::CONTENT_TYPE,
							hyper::header::HeaderValue::from_static("text/html"),
						);
					}

					Ok(Some(index_content))
				} else {
					tracing::info!(
						path = ?path,
						"file not found, returning 404"
					);

					*response = std::mem::take(response).status(hyper::StatusCode::NOT_FOUND);
					Ok(Some("Not Found".into()))
				}
			}
		}
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
		tracing::info!(
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
