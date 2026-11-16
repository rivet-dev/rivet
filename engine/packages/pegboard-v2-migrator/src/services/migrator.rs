//! The migrator runs as a cluster singleton beside Epoxy: exactly one storage worker holds the
//! singleton lease, and its tenure runs the migrator state machine until the lease is revoked.
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use rivet_engine_context::EngineContext;
use rivet_env::tokio::task::JoinHandle;
use rivet_singleton_coordinator::{
	authority::Authority, runtime::SingletonServices,
	singleton_coordinator_machine::types::CoordinatorTenure,
};
use rivet_state_machine_ephemeral::Runner;
use tokio::sync::{Mutex, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{
	metrics,
	migrator_machine::{
		MigratorStateMachine, event::Event, executor::MigratorExecutor, snapshot::PHASES,
		state::State, types::Settings,
	},
	requests::{self, Replies},
	settings::settings_from_config,
	v2_client::V2Client,
};

/// Keeps the tenure's task across cancellation of the singleton runtime's admission future, so
/// `reconcile_stopped` can cancel and join it before the lease is released.
pub struct MigratorServices {
	ctx: EngineContext,
	active: Mutex<Option<Active>>,
}

struct Active {
	stop: CancellationToken,
	joined: JoinHandle<Result<()>>,
}

impl MigratorServices {
	pub fn new(ctx: EngineContext) -> Self {
		Self {
			ctx,
			active: Mutex::new(None),
		}
	}
}

#[async_trait::async_trait]
impl SingletonServices for MigratorServices {
	#[tracing::instrument(skip_all)]
	async fn run(&self, authority: Authority) -> Result<()> {
		self.ctx.wait_depot_ready().await;
		let ctx = self.ctx.with_singleton_scope(authority.clone())?;
		let stop = CancellationToken::new();
		let (finished, completion) = oneshot::channel();
		let mut active = self.active.lock().await;
		ensure!(
			active.is_none(),
			"previous migrator tenure has not been joined"
		);

		let task_stop = stop.clone();
		let task_authority = authority.clone();
		let joined = rivet_env::tokio::task::spawn(self.ctx.env().as_ref(), async move {
			let result = start(ctx.clone(), task_authority, task_stop).await;
			let closed = ctx.close_singleton_scope().await;
			let result = result.and(closed);
			// Epoxy shares the tenure, so a failed migrator stays stopped rather than giving the
			// tenure up.
			if let Err(error) = &result {
				tracing::error!(
					?error,
					"V2 migrator stopped with an error; it stays stopped until the singleton tenure moves"
				);
			}
			let _ = finished.send(result.is_ok());
			result
		});
		*active = Some(Active { stop, joined });
		drop(active);

		rivet_env::select_biased!("revocation stops the tenure before another completion",
			_ = authority.revoked() => {},
			result = completion => {
				if matches!(result, Ok(true)) {
					authority.revoked().await;
				}
			},
		);
		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn reconcile_stopped(&self, _: &CoordinatorTenure) -> Result<()> {
		if let Some(active) = self.active.lock().await.take() {
			active.stop.cancel();
			active.joined.await.context("V2 migrator task failed")??;
		}
		Ok(())
	}
}

/// Runs one tenure of the migrator until `stop` is cancelled or a newer tenure fences it.
///
/// Every write the tenure makes is fenced by its coordinator generation on the migration volume,
/// and every effect is dispatched only while the authority still permits it.
pub async fn start(
	ctx: EngineContext,
	authority: Authority,
	stop: CancellationToken,
) -> Result<()> {
	let tenure = authority.tenure().coordinator_generation;
	let mut config = ctx.config().dynamic_watch();
	let (settings, v2) = load_settings(&ctx).await?;
	tracing::info!(
		tenure,
		enabled = settings.enabled,
		"V2 migrator tenure started"
	);

	let tasks = ctx.env().task_scope();
	let replies = Arc::new(Replies::default());
	let executor = Arc::new(MigratorExecutor::new(
		ctx.clone(),
		authority.clone(),
		v2,
		tasks,
		replies.clone(),
	));
	let runner = Runner::<MigratorStateMachine>::spawn(
		ctx.env().clone(),
		State::new(tenure, settings.clone()),
		executor.clone(),
	);
	// Operator requests reach this tenure only while it runs. A failed subscription leaves the
	// migration running without them.
	let mut serving = true;
	let transport = requests::serve(
		ctx.ups()?,
		authority,
		runner.inbox.downgrade(),
		replies.clone(),
	);
	tokio::pin!(transport);
	let mut snapshot = runner.snapshot.clone();
	let mut current = settings;
	let mut shutdown = ctx.shutdown();

	loop {
		let published = snapshot.borrow_and_update().clone();
		let namespace = current.target_namespace.as_str();
		for (phase, count) in PHASES.iter().zip(published.in_flight_by_phase) {
			metrics::IN_FLIGHT
				.with_label_values(&[namespace, phase])
				.set(count as i64);
		}
		metrics::IN_FLIGHT_OLDEST_STARTED_AT_SECONDS
			.with_label_values(&[namespace])
			.set(
				published
					.oldest_started_at
					.map_or(0, |unix_ms| unix_ms / 1000),
			);
		metrics::IN_FLIGHT_CAPACITY.set(i64::from(current.max_in_flight));
		metrics::COPY_BUDGET_BYTES_PER_SECOND
			.set(i64::try_from(current.max_copy_bytes_per_sec).unwrap_or(i64::MAX));
		metrics::MIGRATE_RATIO
			.with_label_values(&[current.target_namespace.as_str()])
			.set(f64::from(current.migrate_ratio_ppm) / 1_000_000.0);
		if published.stopped {
			break;
		}

		// The tenure ends on its stop token or on process shutdown.
		let stopping = async {
			rivet_env::select_biased!("an explicit stop wins over shutdown",
				_ = stop.cancelled() => {},
				_ = shutdown.recv() => {},
			);
		};
		let event = rivet_env::select_biased!("stop ends the tenure before other wakeups",
			_ = stopping => Wake::Stop,
			changed = config.changed() => match changed {
				Ok(()) => Wake::Config,
				Err(_) => Wake::Stop,
			},
			changed = snapshot.changed() => match changed {
				Ok(()) => Wake::Snapshot,
				Err(_) => Wake::Stop,
			},
			result = rivet_env::tokio::when(serving, &mut transport) => Wake::TransportEnded(result),
		);
		match event {
			Wake::Stop => break,
			Wake::Snapshot => {}
			Wake::TransportEnded(result) => {
				serving = false;
				if let Err(error) = result {
					tracing::error!(
						?error,
						"the V2 migrator stopped answering operator requests; the migration continues, and requests work again on the next tenure"
					);
				}
			}
			Wake::Config => match load_settings(&ctx).await {
				Ok((settings, v2)) => {
					executor.set_v2(v2);
					if settings != current {
						current = settings.clone();
						let _ = runner.inbox.send(Event::SettingsChanged { settings }).await;
					}
				}
				Err(error) => tracing::warn!(?error, "ignoring invalid migrate_from_v2 settings"),
			},
		}
	}

	let _ = runner.inbox.send(Event::StopRequest).await;
	replies.close();
	executor.cancel();
	executor.shutdown().await;
	drop(runner.inbox);
	runner
		.stopped
		.await
		.context("V2 migrator runner failed during shutdown")?;
	tracing::info!(tenure, "V2 migrator tenure stopped");
	Ok(())
}

enum Wake {
	Stop,
	Config,
	Snapshot,
	TransportEnded(Result<()>),
}

/// Reads the settings from the dynamic config, and the target namespace's volume group from its
/// standing, so new entries are placed, and keyed candidates take their id, as a fresh create
/// there would.
async fn load_settings(ctx: &EngineContext) -> Result<(Settings, Option<V2Client>)> {
	let dynamic = ctx.config().dynamic();
	let target_group = match dynamic.migrate_from_v2() {
		Some(_) => crate::v3_actor::target(ctx).await?.group_id(),
		None => None,
	};
	let settings = settings_from_config(&dynamic, ctx.config().dc_label(), target_group);
	let v2 = match dynamic.migrate_from_v2() {
		Some(migration) if !migration.api_urls.is_empty() => {
			Some(V2Client::new(ctx.external_http(), migration)?)
		}
		Some(_) | None => None,
	};
	Ok((settings, v2))
}
