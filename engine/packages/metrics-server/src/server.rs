use std::{convert::Infallible, net::SocketAddr, result::Result::Ok};

use anyhow::*;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{
	Request, Response, body::Incoming, header::CONTENT_TYPE, server::conn::http1,
	service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use rivet_metrics::prometheus::{Encoder, TextEncoder};
use tokio::net::TcpListener;

#[allow(clippy::disallowed_methods, reason = "metrics endpoint is outside Env")]
#[tracing::instrument(skip_all)]
pub async fn run_standalone(config: rivet_config::Config) -> Result<()> {
	let host = config.metrics.host();
	let port = config.metrics.port();
	let addr = SocketAddr::from((host, port));

	let listener = match TcpListener::bind(&addr).await {
		Ok(x) => x,
		Err(err) => {
			tracing::error!(?host, ?port, ?err, "failed to bind metrics server");

			// Hard crash the program since a server failing to bind is critical.
			std::process::exit(1);
		}
	};

	tracing::debug!(?host, ?port, "started metrics server");

	loop {
		let stream = match listener.accept().await {
			Ok((stream, _)) => stream,
			Err(err) => {
				tracing::warn!(
					?err,
					"failed to accept metrics connection, likely fd exhaustion"
				);
				continue;
			}
		};
		let io = TokioIo::new(stream);

		tokio::spawn(async move {
			if let Err(err) = http1::Builder::new()
				.timer(TokioTimer::new())
				.serve_connection(io, service_fn(serve_req))
				.await
			{
				tracing::debug!(?err, "metrics connection failed");
			}
		});
	}
}

#[tracing::instrument(level = "debug", skip_all)]
async fn serve_req(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
	let encoder = TextEncoder::new();

	let metric_families = rivet_metrics::REGISTRY.gather();
	let mut buffer = Vec::new();
	encoder
		.encode(&metric_families, &mut buffer)
		.expect("encode");

	let response = Response::builder()
		.status(200)
		.header(CONTENT_TYPE, encoder.format_type())
		.body(Full::new(Bytes::from(buffer)))
		.expect("response");

	Ok(response)
}
