use std::sync::Arc;

use axum::Router;
use rivet_engine_context::EngineContext;

/// Build internal APIs without starting services or capturing an unready Depot client.
/// The Engine owns the shared listener, including UPS and Epoxy transport routes.
pub fn router(ctx: EngineContext) -> anyhow::Result<Router> {
	let client_ctx = ctx.clone();
	let client: Arc<dyn Fn() -> depot_client::Client + Send + Sync> =
		Arc::new(move || client_ctx.depot());
	let authority = ctx.clone();
	let operator = crate::operator::router(ctx.clone()).merge(crate::repair::router(ctx.clone()));
	Ok(operator.merge(
		crate::volumes::router(client.clone(), ctx.config().api_peer().volumes.clone())?.merge(
			crate::filesystem::router(crate::filesystem::PeerService {
				client,
				clock: ctx.env().clock(),
				region: ctx.config().topology().datacenter_label,
				timeout_ms: ctx.config().depot().volume_request_timeout_ms(),
				may_serve: Arc::new(move || authority.topology().may_reply().unwrap_or(false)),
			}),
		),
	))
}
