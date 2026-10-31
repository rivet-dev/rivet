//! Peer HTTP listener for worker-to-worker requests.
//!
//! Workers reach each other through UPS peer requests, so every registered worker serves
//! the internal volume, UPS and regional Epoxy routers on its advertised API-peer address.

use std::net::SocketAddr;

use anyhow::Result;
use rivet_config::Config;
use rivet_engine_context::EngineContext;

/// Serves internal APIs and peer transports until accepted requests have drained.
///
/// UPS drains requests while this listener still serves them, and finishes shutting down
/// only after the services future, including this listener, returns. Waiting for full UPS
/// shutdown here would hold process shutdown until the drain deadline.
#[tracing::instrument(skip_all)]
pub async fn serve(
	config: Config,
	ctx: EngineContext,
	epoxy: Option<std::sync::Arc<crate::epoxy_services::EpoxyServices>>,
) -> Result<()> {
	let ups = ctx.ups()?;
	let addr = SocketAddr::from((config.api_peer().host()?, config.api_peer().port()));

	let router = ups
		.router()
		.merge(crate::epoxy_peer::router(ctx.clone(), epoxy))
		.merge(rivet_api_peer::create_router(ctx.clone())?);

	let listener = ctx.http_server().bind(addr).await?;
	tracing::info!(%addr, "peer listener bound");
	ups.mark_listening();

	let drained = ups.clone();
	listener
		.serve(
			router,
			Box::pin(async move { drained.requests_drained().await }),
		)
		.await
}
