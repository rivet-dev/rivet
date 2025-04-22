use std::net::SocketAddr;

use hyper::{
	header::CONTENT_TYPE,
	service::{make_service_fn, service_fn},
	Body, Request, Response, Server,
};
use prometheus::{Encoder, TextEncoder};

#[tracing::instrument(level = "debug", skip_all)]
pub async fn run_standalone(port: u16) -> anyhow::Result<()> {
	let addr = SocketAddr::from(([0, 0, 0, 0], port));

	let server = Server::try_bind(&addr)?;

	server
		.serve(make_service_fn(|_| async {
			Ok::<_, hyper::Error>(service_fn(serve_req))
		}))
		.await
		.map_err(Into::into)
}

#[tracing::instrument(level = "debug", skip_all)]
async fn serve_req(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
	let encoder = TextEncoder::new();

	let metric_families = super::registry::REGISTRY.gather();
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
