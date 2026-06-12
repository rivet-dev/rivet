use std::{future::Future, sync::Arc, time::Duration};

use anyhow::Result;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::{RoutingFn, request_context::RequestContext};
use rivet_metrics::prometheus::HistogramVec;
use rivet_perf::{perf_finish, perf_start};

use crate::{errors, metrics, shared_state::SharedState};

pub mod actor_path;
mod api_public;
mod envoy;
pub mod pegboard_gateway;
mod runner;
mod ws_health;

pub(crate) const X_RIVET_TARGET: HeaderName = HeaderName::from_static("x-rivet-target");
pub(crate) const X_RIVET_TOKEN: HeaderName = HeaderName::from_static("x-rivet-token");
pub(crate) const X_RIVET_SKIP_READY_WAIT: HeaderName =
	HeaderName::from_static("x-rivet-skip-ready-wait");
pub(crate) const SEC_WEBSOCKET_PROTOCOL: HeaderName =
	HeaderName::from_static("sec-websocket-protocol");
pub(crate) const WS_PROTOCOL_TARGET: &str = "rivet_target.";
pub(crate) const WS_PROTOCOL_ACTOR: &str = "rivet_actor.";
pub(crate) const WS_PROTOCOL_TOKEN: &str = "rivet_token.";
pub(crate) const WS_PROTOCOL_SKIP_READY_WAIT: &str = "rivet_skip_ready_wait";

const SLOW_PHASE_WARN_THRESHOLD: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub(crate) struct Phase {
	name: &'static str,
	duration: &'static HistogramVec,
	namespace_id: Option<String>,
	actor_id: Option<String>,
	router: Option<&'static str>,
}

impl Phase {
	pub(crate) fn new(name: &'static str, duration: &'static HistogramVec) -> Self {
		Self {
			name,
			duration,
			namespace_id: None,
			actor_id: None,
			router: None,
		}
	}

	pub(crate) fn with_namespace_id(mut self, namespace_id: Id) -> Self {
		self.namespace_id = Some(namespace_id.to_string());
		self
	}

	pub(crate) fn with_actor_id(mut self, actor_id: Id) -> Self {
		self.actor_id = Some(actor_id.to_string());
		self
	}

	pub(crate) fn with_router(mut self, router: &'static str) -> Self {
		self.router = Some(router);
		self
	}
}

pub(crate) async fn phase_timeout<T, E, Fut, ErrFn>(
	phase: Phase,
	budget: Duration,
	fut: Fut,
	err_fn: ErrFn,
) -> Result<T>
where
	Fut: Future<Output = std::result::Result<T, E>>,
	E: Into<anyhow::Error>,
	ErrFn: FnOnce(Duration, Duration) -> anyhow::Error,
{
	let started_at = std::time::Instant::now();
	let namespace_id = phase.namespace_id.as_deref().unwrap_or("");
	let measure = perf_start!(
		phase.duration,
		slow_ms = SLOW_PHASE_WARN_THRESHOLD.as_millis() as u64,
		"guard_route_phase",
		labels: { namespace_id = %namespace_id },
		fields: {
			phase = %phase.name,
			actor_id = ?phase.actor_id,
			router = ?phase.router,
		},
	);
	let res = tokio::time::timeout(budget, fut).await;
	let elapsed = started_at.elapsed();
	perf_finish!(measure, fields: { timeout_ms = budget.as_millis() as u64 });

	res.map_err(|_| err_fn(elapsed, budget))?
		.map_err(Into::into)
}

fn route_dispatch_timeout(
	router: &'static str,
	elapsed: Duration,
	timeout: Duration,
) -> anyhow::Error {
	errors::RouteDispatchTimeout {
		router: router.to_string(),
		elapsed_ms: elapsed.as_millis() as u64,
		timeout_ms: timeout.as_millis() as u64,
	}
	.build()
}

fn route_dispatch_phase(router: &'static str) -> Phase {
	Phase::new("route_dispatch", &metrics::ROUTE_DISPATCH_DURATION).with_router(router)
}

/// Creates the main routing function that handles all incoming requests
#[tracing::instrument(skip_all)]
pub fn create_routing_function(ctx: &StandaloneCtx, shared_state: SharedState) -> RoutingFn {
	let ctx = ctx.clone();
	Arc::new(move |req_ctx| {
		let ctx = ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id()).unwrap();
		let shared_state = shared_state.clone();
		let hostname = req_ctx.hostname().to_string();
		let path = req_ctx.path().to_string();

		Box::pin(
			async move {
				tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "Routing request");

				if ws_health::matches_path(req_ctx.path()) {
					if ctx.config().guard().enable_websocket_health_route() {
						metrics::ROUTE_TOTAL.with_label_values(&["ws_health"]).inc();
						return Ok(ws_health::route_request());
					}

					metrics::ROUTE_TOTAL.with_label_values(&["none"]).inc();

					return Err(errors::NoRoute {
						host: req_ctx.hostname().to_string(),
						path: req_ctx.path().to_string(),
					}
					.build());
				}

				// MARK: Path-based routing

				// Route actor
				if let Some(routing_output) = phase_timeout(
					route_dispatch_phase("pegboard_path"),
					ctx.config().guard().route_dispatch_timeout(),
					pegboard_gateway::route_request_path_based(&ctx, &shared_state, req_ctx),
					|elapsed, timeout| route_dispatch_timeout("pegboard_path", elapsed, timeout),
				)
				.await?
				{
					metrics::ROUTE_TOTAL.with_label_values(&["gateway"]).inc();

					return Ok(routing_output);
				}

				// Route runner
				if let Some(routing_output) = phase_timeout(
					route_dispatch_phase("runner_path"),
					ctx.config().guard().route_dispatch_timeout(),
					runner::route_request_path_based(&ctx, req_ctx),
					|elapsed, timeout| route_dispatch_timeout("runner_path", elapsed, timeout),
				)
				.await?
				{
					metrics::ROUTE_TOTAL.with_label_values(&["runner"]).inc();

					return Ok(routing_output);
				}

				// Route envoy
				if let Some(routing_output) = phase_timeout(
					route_dispatch_phase("envoy_path"),
					ctx.config().guard().route_dispatch_timeout(),
					envoy::route_request_path_based(&ctx, req_ctx),
					|elapsed, timeout| route_dispatch_timeout("envoy_path", elapsed, timeout),
				)
				.await?
				{
					metrics::ROUTE_TOTAL.with_label_values(&["envoy"]).inc();

					return Ok(routing_output);
				}

				// MARK: Header- & protocol-based routing (X-Rivet-Target)
				// Determine target
				let target = if req_ctx.is_websocket() {
					// For WebSocket, parse the sec-websocket-protocol header
					req_ctx
						.headers()
						.get(SEC_WEBSOCKET_PROTOCOL)
						.and_then(|protocols| protocols.to_str().ok())
						.and_then(|protocols| {
							// Parse protocols to find target.{value}
							protocols
								.split(',')
								.map(|p| p.trim())
								.find_map(|p| p.strip_prefix(WS_PROTOCOL_TARGET))
								.map(ToOwned::to_owned)
						})
				} else {
					// For HTTP, use the x-rivet-target header
					req_ctx
						.headers()
						.get(X_RIVET_TARGET)
						.and_then(|x| x.to_str().ok())
						.map(ToOwned::to_owned)
				};

				// Read target
				if let Some(target) = &target {
					if let Some(routing_output) = phase_timeout(
						route_dispatch_phase("pegboard_header"),
						ctx.config().guard().route_dispatch_timeout(),
						pegboard_gateway::route_request(&ctx, &shared_state, req_ctx, &target),
						|elapsed, timeout| {
							route_dispatch_timeout("pegboard_header", elapsed, timeout)
						},
					)
					.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["gateway"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) = phase_timeout(
						route_dispatch_phase("runner_header"),
						ctx.config().guard().route_dispatch_timeout(),
						runner::route_request(&ctx, req_ctx, &target),
						|elapsed, timeout| {
							route_dispatch_timeout("runner_header", elapsed, timeout)
						},
					)
					.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["runner"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) = phase_timeout(
						route_dispatch_phase("envoy_header"),
						ctx.config().guard().route_dispatch_timeout(),
						envoy::route_request(&ctx, req_ctx, target),
						|elapsed, timeout| route_dispatch_timeout("envoy_header", elapsed, timeout),
					)
					.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["envoy"]).inc();

						return Ok(routing_output);
					}

					if let Some(routing_output) = phase_timeout(
						route_dispatch_phase("api_public_header"),
						ctx.config().guard().route_dispatch_timeout(),
						api_public::route_request(&ctx, &target),
						|elapsed, timeout| {
							route_dispatch_timeout("api_public_header", elapsed, timeout)
						},
					)
					.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["api"]).inc();

						return Ok(routing_output);
					}
				} else {
					// No x-rivet-target header, try routing to api-public by default
					if let Some(routing_output) = phase_timeout(
						route_dispatch_phase("api_public_default"),
						ctx.config().guard().route_dispatch_timeout(),
						api_public::route_request(&ctx, "api-public"),
						|elapsed, timeout| {
							route_dispatch_timeout("api_public_default", elapsed, timeout)
						},
					)
					.await?
					{
						metrics::ROUTE_TOTAL.with_label_values(&["api"]).inc();

						return Ok(routing_output);
					}
				}

				metrics::ROUTE_TOTAL.with_label_values(&["none"]).inc();

				tracing::debug!(hostname=%req_ctx.hostname(), path=%req_ctx.path(), "No route found");
				Err(errors::NoRoute {
					host: req_ctx.hostname().to_string(),
					path: req_ctx.path().to_string(),
				}
				.build())
			}
			.instrument(tracing::info_span!("routing_fn", %hostname, %path)),
		)
	})
}

/// Validates that the request hostname is valid for the current datacenter.
/// Returns an error if the host does not match a valid regional host.
pub(crate) fn validate_regional_host(ctx: &StandaloneCtx, req_ctx: &RequestContext) -> Result<()> {
	let current_dc = ctx.config().topology().current_dc()?;
	if !current_dc.is_valid_regional_host(req_ctx.hostname()) {
		tracing::warn!(
			hostname = %req_ctx.hostname(),
			datacenter = ?current_dc.name,
			"invalid host for current datacenter"
		);

		let valid_hosts = if let Some(hosts) = &current_dc.valid_hosts {
			hosts.join(", ")
		} else {
			current_dc
				.public_url
				.host_str()
				.map(|h| h.to_string())
				.unwrap_or_else(|| "unknown".to_string())
		};

		return Err(errors::MustUseRegionalHost {
			host: req_ctx.hostname().to_string(),
			datacenter: current_dc.name.clone(),
			valid_hosts,
		}
		.build());
	}

	Ok(())
}
