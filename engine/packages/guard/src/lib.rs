use anyhow::Result;
use axum::{Router, extract::Json, routing::get};
use hyper::service::service_fn;
use hyper::{
	Request, Response,
	body::Incoming,
	header::{HeaderName, HeaderValue},
};
use rivet_engine_context::EngineContext;
use rivet_guard_core::{ProxyState, RequestContext, ResponseBody, X_RIVET_RAY_ID};
use rivet_util::rng::RngStreamIdExt;
use std::{net::SocketAddr, sync::Arc};
use tokio_util::sync::CancellationToken;
use tower::util::ServiceExt;
use tracing::Instrument;

mod metrics;

pub const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

/// Runtime name reported by `/health` and `rivet-engine --runtime-info`.
pub const ENGINE_RUNTIME: &str = "engine";

/// Architecture reported by `/health` and `rivet-engine --runtime-info`. Local RivetKit hosts
/// refuse engines that do not report it, because v2 engines report the same runtime and version.
pub const ENGINE_ARCHITECTURE: &str = "v3";

pub async fn start(ctx: EngineContext) -> Result<()> {
	let Some(_ingress) = ctx.ingress().enter() else {
		return Ok(());
	};
	let envoys = pegboard_envoy::service(&ctx);
	let _shutdown = envoys.shutdown_owner();
	let result = serve(ctx, envoys.clone()).await;
	// Error paths must also release and join accepted upgrades before storage closes.
	envoys.drain_connections().await;
	result
}

async fn serve(ctx: EngineContext, envoys: pegboard_envoy::Service) -> Result<()> {
	// Shared state between all requests
	let proxy_state = ProxyState::new(&ctx);
	let start_waits = pegboard_gateway::start_waits(&ctx);
	let router = Router::new()
		.route(
			"/health",
			get(|| async {
				Json(serde_json::json!({
					"runtime": ENGINE_RUNTIME,
					"status": "ok",
					"version": env!("CARGO_PKG_VERSION"),
					// Local hosts refuse to reuse an engine without this field, because v2
					// engines report the same runtime and version.
					"architecture": ENGINE_ARCHITECTURE,
				}))
			}),
		)
		.merge(pegboard_gateway::router(&ctx, &start_waits))
		.merge(pegboard_envoy::router(&ctx, &envoys))
		.merge(rivet_api_public_shim::router(&ctx));

	// Background services of the routers, cancelled and joined at shutdown.
	let services = ctx.env().task_scope();
	let lost_checks = pegboard_envoy::services::lost_checks::start(ctx.clone(), envoys.clone());
	services.spawn(async move {
		if let Err(error) = lost_checks.await {
			tracing::error!(?error, "envoy lost check service stopped");
		}
	});
	// Set up HTTP server
	let http_addr: std::net::SocketAddr =
		(ctx.config().guard().host(), ctx.config().guard().port()).into();
	let listener = ctx.http_server().bind(http_addr).await?;
	// hyper spawns a task per HTTP/2 stream through this executor; keep it on tokio so the
	// request path is the same as before the listener moved behind `Env`.
	#[allow(
		clippy::disallowed_methods,
		reason = "hyper's per-stream executor stays on tokio for the request hot path"
	)]
	let server = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
	let graceful = Arc::new(hyper_util::server::graceful::GracefulShutdown::new());
	let mut shutdown = ctx.shutdown();

	tracing::info!("HTTP server listening on {}", http_addr);

	// Accept connections until we receive a shutdown signal
	let accept: rivet_env::capabilities::http::AcceptConnection = {
		let ctx = ctx.clone();
		let graceful = graceful.clone();
		Box::new(move |connection, remote_addr| {
			process_connection(
				&ctx,
				&proxy_state,
				&router,
				&server,
				&graceful,
				connection,
				remote_addr,
			)
		})
	};
	let stopped = Box::pin(async move {
		if let Err(error) = shutdown.recv().await {
			tracing::warn!(%error, "shutdown signal failed; stopping guard");
		}
	});
	listener.serve_connections(accept, stopped).await?;

	// The listener dropped its handle to the connection watcher when it stopped.
	match Arc::try_unwrap(graceful) {
		Ok(graceful) => graceful.shutdown().await,
		Err(_) => {
			tracing::warn!("guard connections still referenced at shutdown; not draining them")
		}
	}
	envoys.drain_connections().await;
	services.shutdown().await;

	tracing::info!("guard shutdown complete");

	Ok(())
}

// Helper function to process regular connections
#[tracing::instrument(skip_all, fields(?remote_addr))]
fn process_connection(
	ctx: &EngineContext,
	proxy_state: &ProxyState,
	router: &Router,
	server: &hyper_util::server::conn::auto::Builder<hyper_util::rt::TokioExecutor>,
	graceful: &hyper_util::server::graceful::GracefulShutdown,
	connection: rivet_env::capabilities::http::Connection,
	remote_addr: SocketAddr,
) {
	if ctx.config().guard().tcp_nodelay()
		&& let Err(err) = connection.set_nodelay(true)
	{
		tracing::debug!(?err, "failed to enable tcp nodelay");
	}

	let io = hyper_util::rt::TokioIo::new(connection);
	let client_disconnect_token = CancellationToken::new();

	// Using service_fn to convert our function into a hyper service
	let service = {
		let ctx = ctx.clone();
		let proxy_state = proxy_state.clone();
		let router = router.clone();
		let client_disconnect_token = client_disconnect_token.clone();

		service_fn(move |req| {
			let ctx = ctx.clone();
			let proxy_state = proxy_state.clone();
			let router = router.clone();
			let client_disconnect_token = client_disconnect_token.clone();

			async move {
				serve_request(
					ctx,
					proxy_state,
					router,
					remote_addr,
					client_disconnect_token,
					req,
				)
				.await
			}
		})
	};

	// Serve the connection with graceful shutdown support
	let conn = server.serve_connection_with_upgrades(io, service);
	let conn = graceful.watch(conn.into_owned());

	ctx.env().spawn(Box::pin(
		async move {
			if let Err(err) = conn.await {
				tracing::warn!("HTTP connection error: {}", err);
			}

			client_disconnect_token.cancel();

			tracing::debug!("HTTP connection dropped: {}", remote_addr);
		}
		.instrument(tracing::debug_span!(parent: None, "process_connection_task")),
	));
}

async fn serve_request(
	ctx: EngineContext,
	proxy_state: ProxyState,
	router: Router,
	remote_addr: SocketAddr,
	client_disconnect_token: CancellationToken,
	mut req: Request<Incoming>,
) -> Result<Response<ResponseBody>> {
	#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
	let start_time = std::time::Instant::now();
	let ray_id = ctx.env().rng("guard.ray-id").id_v1(ctx.config().dc_label());

	// Extract IP address from X-Forwarded-For header or fall back to remote_addr
	let client_ip = req
		.headers()
		.get(X_FORWARDED_FOR)
		.and_then(|h| h.to_str().ok())
		.and_then(|forwarded| {
			// X-Forwarded-For can be a comma-separated list, take the first IP
			forwarded.split(',').next().map(|s| s.trim())
		})
		.and_then(|ip_str| ip_str.parse::<std::net::IpAddr>().ok())
		.unwrap_or_else(|| remote_addr.ip());
	let is_websocket = hyper_tungstenite::is_upgrade_request(&req);

	let req_ctx = RequestContext::new(
		proxy_state,
		remote_addr,
		ray_id,
		req.uri().clone(),
		req.method().clone(),
		req.headers().clone(),
		is_websocket,
		client_ip,
		start_time,
		client_disconnect_token,
	);
	req.extensions_mut().insert(req_ctx);

	// Route request
	let Ok(mut res) = router.oneshot(req).await;

	// Clients report the ray id with their errors so a failed request can be traced.
	match HeaderValue::from_str(&ray_id.to_string()) {
		Ok(value) => {
			res.headers_mut().insert(X_RIVET_RAY_ID, value);
		}
		Err(error) => {
			tracing::warn!(?error, %ray_id, "ray id is not a valid header value");
		}
	}

	let kind = if is_websocket {
		"websocket_upgrade"
	} else {
		"http"
	};
	metrics::REQUEST_SECONDS
		.with_label_values(&[kind])
		.observe(start_time.elapsed().as_secs_f64());

	Ok(res.map(ResponseBody::Axum))
}
