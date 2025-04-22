use global_error::prelude::*;
use hyper::{
	header::CONTENT_TYPE,
	service::{make_service_fn, service_fn},
	Body, Request, Response, Server,
};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

// TODO: Record extra labels

#[tracing::instrument(skip_all)]
pub async fn run_standalone(config: rivet_config::Config) -> GlobalResult<()> {
	let host = config.server()?.rivet.metrics.host();
	let port = config.server()?.rivet.metrics.port();
	let addr = SocketAddr::from((host, port));

	let server = match Server::try_bind(&addr) {
		Ok(x) => x,
		Err(err) => {
			tracing::error!(?host, ?port, ?err, "failed to bind metrics server");

			// TODO: Find cleaner way of crashing entire program
			// Hard crash program since a server failing to bind is critical
			std::process::exit(1);
		}
	};

	let server = server.serve(make_service_fn(|_| async {
		Ok::<_, hyper::Error>(service_fn(serve_req))
	}));

	tracing::info!(?host, ?port, "started metrics server");
	server.await?;

	Ok(())
}

#[tracing::instrument(level="debug", skip_all)]
async fn serve_req(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
	let encoder = TextEncoder::new();

	let metric_families = crate::registry::REGISTRY.gather();
	let mut buffer = Vec::new();
	encoder
		.encode(&metric_families, &mut buffer)
		.expect("encode");

	let response = Response::builder()
		.status(200)
		.header(CONTENT_TYPE, encoder.format_type())
		.body(Body::from(buffer))
		.expect("response");

	Ok(response)
}
