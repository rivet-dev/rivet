use std::{net::SocketAddr, result::Result::Ok};

use anyhow::*;
use hyper::{
	Body, Request, Response, Server,
	header::CONTENT_TYPE,
	server::{Builder, conn::AddrIncoming},
	service::{make_service_fn, service_fn},
};
use rivet_metrics::prometheus::{Encoder, TextEncoder};

#[tracing::instrument(skip_all)]
pub async fn run_standalone(config: rivet_config::Config) -> Result<()> {
	let host = config.metrics.host();
	let port = config.metrics.port();
	let addr = SocketAddr::from((host, port));

	let server = match bind_server(addr) {
		Ok(x) => x,
		Err(err) => {
			tracing::error!(?host, ?port, ?err, "failed to bind metrics server");

			// Hard crash the program since a server failing to bind is critical.
			std::process::exit(1);
		}
	};

	let server = server.serve(make_service_fn(|_| async {
		Ok::<_, hyper::Error>(service_fn(serve_req))
	}));

	tracing::debug!(?host, ?port, "started metrics server");
	server.await?;

	Ok(())
}

fn bind_server(addr: SocketAddr) -> Result<Builder<AddrIncoming>> {
	let listener = rivet_util::net::bind_std_tcp_listener(addr)?;
	Ok(Server::from_tcp(listener)?)
}

#[tracing::instrument(level = "debug", skip_all)]
async fn serve_req(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
	let encoder = TextEncoder::new();

	let metric_families = rivet_metrics::REGISTRY.gather();
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
