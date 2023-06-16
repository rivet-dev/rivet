use std::net::SocketAddr;

use hyper::{
	header::CONTENT_TYPE,
	service::{make_service_fn, service_fn},
	Body, Request, Response, Server,
};
use prometheus::{Encoder, TextEncoder};

// TODO: Record extra labels

#[tracing::instrument(skip_all)]
pub async fn run_standalone() {
	let port: u16 = std::env::var("METRICS_PORT")
		.ok()
		.and_then(|v| v.parse::<u16>().ok())
		.expect("METRICS_PORT");
	let addr = SocketAddr::from(([0, 0, 0, 0], port));

	let server = Server::bind(&addr).serve(make_service_fn(|_| async {
		Ok::<_, hyper::Error>(service_fn(serve_req))
	}));
	if let Err(err) = server.await {
		tracing::error!(?err, "server error");
	}
}

#[tracing::instrument(skip_all)]
async fn serve_req(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
	let encoder = TextEncoder::new();

	let metric_families = crate::registry::REGISTRY.gather();
	let mut buffer = Vec::new();
	encoder.encode(&metric_families, &mut buffer).expect("encode");

	let response = Response::builder()
		.status(200)
		.header(CONTENT_TYPE, encoder.format_type())
		.body(Body::from(buffer))
		.expect("response");

	Ok(response)
}
