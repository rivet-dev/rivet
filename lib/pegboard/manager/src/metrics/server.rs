use std::net::SocketAddr;

use hyper::{
	header::CONTENT_TYPE,
	service::{make_service_fn, service_fn},
	Body, Request, Response, Server,
};
use prometheus::{Encoder, TextEncoder};

const METRICS_PORT: u16 = 6000;

#[tracing::instrument(skip_all)]
pub async fn run_standalone() {
	let addr = SocketAddr::from(([0, 0, 0, 0], METRICS_PORT));

	let server = Server::bind(&addr).serve(make_service_fn(|_| async {
		Ok::<_, hyper::Error>(service_fn(serve_req))
	}));
	if let Err(err) = server.await {
		tracing::error!(?err, "metrics server error");
	}
}

#[tracing::instrument(skip_all)]
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
