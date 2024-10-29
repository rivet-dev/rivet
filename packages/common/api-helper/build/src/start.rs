use global_error::prelude::*;
use hyper::{
	body::HttpBody,
	server::conn::AddrStream,
	service::{make_service_fn, service_fn},
	Body, Request, Response, Server,
};
use std::{
	convert::Infallible,
	future::Future,
	net::{IpAddr, SocketAddr},
	time::Instant,
};
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(skip_all)]
pub async fn start<T: 'static, Fut>(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	host: IpAddr,
	port: u16,
	handle: T,
) -> GlobalResult<()>
where
	T: Fn(
			chirp_client::SharedClientHandle,
			rivet_config::Config,
			rivet_pools::Pools,
			rivet_cache::Cache,
			Uuid,
			Request<Body>,
		) -> Fut
		+ Send
		+ Sync
		+ Copy,
	Fut: Future<Output = Result<Response<Body>, http::Error>> + Send,
{
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;

	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;

	// A `MakeService` that produces a `Service` to handle each connection
	let make_service = make_service_fn(move |conn: &AddrStream| {
		let shared_client = shared_client.clone();
		let config = config.clone();
		let pools = pools.clone();
		let cache = cache.clone();

		// Create a `Service` for responding to the request
		let remote_addr = conn.remote_addr();
		let service = service_fn(move |mut req: Request<Body>| {
			let start = Instant::now();

			let shared_client = shared_client.clone();
			let config = config.clone();
			let pools = pools.clone();
			let cache = cache.clone();

			// Add the SocketAddr as an extension to the request
			req.extensions_mut().insert(remote_addr);

			// Handle request
			let ray_id = Uuid::new_v4();
			let req_span = tracing::info_span!("http request", method = %req.method(), uri = %req.uri(), %ray_id);
			async move {
				tracing::info!(
					method = %req.method(),
					uri = %req.uri(),
					headers = ?req.headers(),
					body_size_hint = ?req.body().size_hint(),
					remote_addr = %remote_addr,
					"http request meta"
				);

				let res = tokio::task::Builder::new()
					.name("api_helper::handle")
					.spawn(
						async move {
							let mut res =
								handle(shared_client, config, pools, cache, ray_id, req).await?;
							res.headers_mut()
								.insert("rvt-ray-id", ray_id.to_string().parse()?);
							Result::<Response<Body>, http::Error>::Ok(res)
						}
						.in_current_span(),
					);
				let res = match res {
					Ok(res) => match res.await {
						Ok(res) => match res {
							Ok(res) => res,
							Err(err) => {
								tracing::error!(?err, "http error");
								return Err(err);
							}
						},
						Err(_) => {
							tracing::error!("http error");
							return Ok(Response::builder()
								.status(http::StatusCode::INTERNAL_SERVER_ERROR)
								.body(Body::empty())?);
						}
					},
					Err(err) => {
						tracing::error!(?err, "tokio spawn error");
						return Ok(Response::builder()
							.status(http::StatusCode::INTERNAL_SERVER_ERROR)
							.body(Body::empty())?);
					}
				};

				if res.status().is_server_error() {
					tracing::error!(status = ?res.status().as_u16(), "http server error");
				} else if res.status().is_client_error() {
					tracing::warn!(status = ?res.status().as_u16(), "http client error");
				} else if res.status().is_redirection() {
					tracing::info!(status = ?res.status().as_u16(), "http redirection");
				} else if res.status().is_informational() {
					tracing::info!(status = ?res.status().as_u16(), "http informational");
				}

				let duration = start.elapsed().as_secs_f64();
				tracing::info!(
					status = %res.status().as_u16(),
					headers = ?res.headers(),
					body_size_hint = ?res.body().size_hint(),
					duration = duration,
					"http response meta"
				);

				Ok::<_, http::Error>(res)
			}
			.instrument(req_span)
		});

		// Return the service to hyper
		async move { Ok::<_, Infallible>(service) }
	});

	let addr = SocketAddr::from((host, port));
	let server = match Server::try_bind(&addr) {
		Ok(x) => x,
		Err(err) => {
			tracing::error!(?host, ?port, ?err, "failed to bind api server");

			// TODO: Find cleaner way of crashing entire program
			// Hard crash program since a server failing to bind is critical
			std::process::exit(1);
		}
	};

	tracing::info!(?host, ?port, "server listening");
	server.serve(make_service).await?;

	Ok(())
}
