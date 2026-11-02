//! Epoxy's background services run under the storage worker's existing singleton election.
//! Membership is fixed to this datacenter, so a tenure owns only peer version discovery.

use anyhow::{Context, Result, ensure};
use rivet_engine_context::EngineContext;
use rivet_env::tokio::task::JoinHandle;
use rivet_singleton_coordinator::{
	authority::Authority, runtime::SingletonServices,
	singleton_coordinator_machine::types::CoordinatorTenure,
};
use tokio::sync::{Mutex, oneshot, watch};
use tokio_util::sync::CancellationToken;

/// Retains service tasks across cancellation of the singleton runtime's admission future.
pub(crate) struct EpoxyServices {
	ctx: EngineContext,
	active: Mutex<Option<Active>>,
	pub(crate) serving: watch::Sender<Option<(Authority, epoxy::client::Client)>>,
}

struct Active {
	stop: CancellationToken,
	joined: JoinHandle<Result<()>>,
}

impl EpoxyServices {
	pub(crate) fn new(ctx: EngineContext) -> Self {
		Self {
			ctx,
			active: Mutex::new(None),
			serving: watch::channel(None).0,
		}
	}
}

#[async_trait::async_trait]
impl SingletonServices for EpoxyServices {
	#[tracing::instrument(skip_all)]
	async fn run(&self, authority: Authority) -> Result<()> {
		// Capture the scoped client only after volume access is installed.
		self.ctx.wait_depot_ready().await;
		let ctx = self.ctx.with_singleton_scope(authority.clone())?;
		let client = epoxy::client::Client::with_depot(ctx.config(), ctx.depot(), ctx.http())?;
		let stop = CancellationToken::new();
		let (finished, completion) = oneshot::channel();
		let mut active = self.active.lock().await;
		ensure!(
			active.is_none(),
			"previous Epoxy tenure has not been joined"
		);

		// The task survives cancellation of run so reconcile_stopped can cancel and join
		// reconciler runners, effects, and discovery before the coordinator reports the scope
		// stopped. Only then does it release the lease, and a step-down revokes the scope a
		// full stop timeout before its deadline, so no successor starts while these run.
		let serving = self.serving.clone();
		let admitted = authority.clone();
		let task_stop = stop.clone();
		let jwt_authority = authority.clone();
		let joined = rivet_env::tokio::task::spawn(self.ctx.env().as_ref(), async move {
			serving.send_replace(Some((admitted, client.clone())));
			let result = run_services(&ctx, client, jwt_authority, task_stop).await;
			serving.send_replace(None);
			let closed = ctx.close_singleton_scope().await;
			let result = result.and(closed);
			if let Err(error) = &result {
				tracing::warn!(?error, "Epoxy services stopped");
			}
			let _ = finished.send(result.is_ok());
			result
		});
		*active = Some(Active { stop, joined });
		drop(active);

		rivet_env::select_biased!("revocation stops the tenure before another completion",
			_ = authority.revoked() => {},
			result = completion => {
				// Graceful process shutdown joins the services before UPS drains. Keep
				// this tenure until the storage owner closes it instead of triggering
				// repeated elections and service starts during that drain.
				if matches!(result, Ok(true)) {
					authority.revoked().await;
				}
			},
		);
		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn reconcile_stopped(&self, _: &CoordinatorTenure) -> Result<()> {
		self.serving.send_replace(None);
		if let Some(active) = self.active.lock().await.take() {
			active.stop.cancel();
			active
				.joined
				.await
				.context("Epoxy services task failed")??;
		}
		Ok(())
	}
}

/// Runs version discovery until the tenure stops, then joins it while scoped clients remain
/// owned.
async fn run_services(
	ctx: &EngineContext,
	client: epoxy::client::Client,
	authority: Authority,
	stop: CancellationToken,
) -> Result<()> {
	client.start_version_discovery().await?;
	let mut shutdown = ctx.shutdown();
	tracing::info!("Epoxy services started");

	// Membership is fixed to this datacenter, so the tenure only owns version discovery until
	// it is stopped or the process shuts down.
	let jwt = rivet_auth_jwt::runtime::run(
		ctx.env().clone(),
		ctx.object_stores().lease.clone(),
		ctx.config().clone(),
		authority,
		stop.clone(),
		ctx.ups()?.clone(),
	);
	let lifetime = async {
		let result = rivet_env::select_biased!("process shutdown ends the tenure",
			result = shutdown.recv() => result.map(|_| ()).map_err(anyhow::Error::from),
			_ = stop.cancelled() => Ok(()),
		);
		stop.cancel();
		result
	};
	let (result, jwt_result) = tokio::join!(lifetime, jwt);
	let closed = client.shutdown().await;
	tracing::info!("Epoxy services joined");
	result.and(jwt_result).and(closed)
}
