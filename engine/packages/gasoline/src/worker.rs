use std::{
	collections::HashMap,
	time::{Duration, Instant},
};

use anyhow::{Context, Result};
use futures_util::{StreamExt, stream::FuturesUnordered};
use opentelemetry::trace::TraceContextExt;
use rivet_runtime::TermSignal;
use rivet_util::Id;
use tokio::{sync::watch, task::JoinHandle};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
	ctx::WorkflowCtx,
	db::{BumpSubSubject, DatabaseHandle},
	error::WorkflowError,
	metrics,
	registry::RegistryHandle,
};

/// How often to run gc and update ping.
pub(crate) const PING_INTERVAL: Duration = Duration::from_secs(10);
/// How often to publish metrics.
const METRICS_INTERVAL: Duration = Duration::from_secs(20);
// How long the pull workflows function can take before shutting down the runtime.
const PULL_WORKFLOWS_TIMEOUT: Duration = Duration::from_secs(10);
const SHUTDOWN_PROGRESS_INTERVAL: Duration = Duration::from_secs(7);

/// Used to spawn a new thread that indefinitely polls the database for new workflows. Only pulls workflows
/// that are registered in its registry. After pulling, the workflows are ran and their state is written to
/// the database.
pub struct Worker {
	worker_id: Id,

	registry: RegistryHandle,
	db: DatabaseHandle,

	config: rivet_config::Config,
	pools: rivet_pools::Pools,

	running_workflows: HashMap<Id, WorkflowHandle>,
}

impl Worker {
	pub fn new(
		registry: RegistryHandle,
		db: DatabaseHandle,
		config: rivet_config::Config,
		pools: rivet_pools::Pools,
	) -> Self {
		Worker {
			worker_id: Id::new_v1(config.dc_label()),

			registry,
			db,

			config,
			pools,

			running_workflows: HashMap::new(),
		}
	}

	/// Polls the database periodically or wakes immediately when `Database::bump_sub` finishes.
	/// Provide a shutdown_rx to allow shutting down without triggering SIGTERM.
	#[tracing::instrument(skip_all, fields(worker_id=%self.worker_id))]
	pub async fn start(mut self, mut shutdown_rx: Option<watch::Receiver<()>>) -> Result<()> {
		tracing::debug!(
			registered_workflows = ?self.registry.size(),
			"started worker",
		);

		let cache = rivet_cache::CacheInner::from_env(&self.config, self.pools.clone())?;

		// We use ready_chunks because multiple bumps in a row should be processed as 1 bump
		let mut bump_sub = self
			.db
			.bump_sub(BumpSubSubject::Worker)
			.await?
			.ready_chunks(1024);

		let mut tick_interval = tokio::time::interval(self.db.worker_poll_interval());
		tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		let mut term_signal = TermSignal::new().await;

		// Update ping at least once before doing anything else
		self.db
			.update_worker_ping(self.worker_id)
			.await
			.context("failed updating worker ping")?;

		// Create handles for bg tasks
		let mut gc_handle = self.gc();
		let mut metrics_handle = self.publish_metrics();

		let res = loop {
			let shutdown_fut = async {
				if let Some(shutdown_rx) = &mut shutdown_rx {
					shutdown_rx.changed().await
				} else {
					std::future::pending().await
				}
			};

			tokio::select! {
				_ = tick_interval.tick() => {},
				res = bump_sub.next() => {
					match res {
						Some(bumps) => {
							metrics::WORKER_BUMPS_PER_TICK
								.with_label_values(&[self.worker_id.to_string().as_str()])
								.observe(bumps.len() as f64);
						}
						None => break Err(WorkflowError::SubscriptionUnsubscribed.into()),
					}

					tick_interval.reset();
				},

				res = &mut gc_handle => {
					tracing::error!(?res, "metrics task unexpectedly stopped");
					break Ok(());
				}
				res = &mut metrics_handle => {
					tracing::error!(?res, "metrics task unexpectedly stopped");
					break Ok(());
				},
				res = shutdown_fut => {
					if res.is_err() {
						tracing::debug!("shutdown channel dropped, ignoring");
						shutdown_rx = None;
					} else {
						break Ok(());
					}
				}
				_ = term_signal.recv() => break Ok(()),
			}

			if let Err(err) = self.tick(&cache).await {
				// Cancel background tasks. We abort because these are not critical tasks.
				gc_handle.abort();
				metrics_handle.abort();

				break Err(err);
			}
		};

		// Cancel background tasks
		gc_handle.abort();
		metrics_handle.abort();

		if let Err(err) = &res {
			tracing::error!(?err, "worker errored, attempting graceful shutdown");
		}

		self.shutdown(term_signal).await;

		res
	}

	/// Query the database for new workflows and run them.
	#[tracing::instrument(skip_all)]
	async fn tick(&mut self, cache: &rivet_cache::Cache) -> Result<()> {
		// Create filter from registered workflow names
		let filter = self
			.registry
			.workflows
			.keys()
			.map(|k| k.as_str())
			.collect::<Vec<_>>();

		// Query awake workflows
		let workflows = tokio::time::timeout(
			PULL_WORKFLOWS_TIMEOUT,
			self.db.pull_workflows(self.worker_id, &filter),
		)
		.await
		.context("took too long pulling workflows, worker cannot continue")??;

		// Remove join handles for completed workflows. This must happen after we pull workflows to ensure an
		// accurate state of the current workflows
		self.running_workflows
			.retain(|_, wf| !wf.handle.is_finished());

		for workflow in workflows {
			let workflow_id = workflow.workflow_id;

			if self.running_workflows.contains_key(&workflow_id) {
				tracing::error!(?workflow_id, "workflow already running");
				continue;
			}

			let (stop_tx, stop_rx) = watch::channel(());
			let name = workflow.workflow_name.clone();

			let ctx = WorkflowCtx::new(
				self.registry.clone(),
				self.db.clone(),
				self.config.clone(),
				self.pools.clone(),
				cache.clone(),
				workflow,
				stop_rx,
			)?;

			let current_span_ctx = tracing::Span::current()
				.context()
				.span()
				.span_context()
				.clone();

			let handle = tokio::spawn(
				// NOTE: No .in_current_span() because we want this to be a separate trace
				async move {
					if let Err(err) = ctx.run(current_span_ctx).await {
						tracing::error!(?err, ?workflow_id, "unhandled workflow error");
					}
				},
			);

			self.running_workflows.insert(
				workflow_id,
				WorkflowHandle {
					name,
					stop: stop_tx,
					handle,
				},
			);
		}

		metrics::WORKER_WORKFLOW_ACTIVE.reset();
		for (_, wf) in &self.running_workflows {
			metrics::WORKER_WORKFLOW_ACTIVE
				.with_label_values(&[self.worker_id.to_string().as_str(), wf.name.as_str()])
				.inc();
		}

		Ok(())
	}

	fn gc(&self) -> JoinHandle<()> {
		let db = self.db.clone();
		let worker_id = self.worker_id;

		tokio::spawn(
			async move {
				let mut ping_interval = tokio::time::interval(PING_INTERVAL);
				ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

				loop {
					ping_interval.tick().await;

					if let Err(err) = db.update_worker_ping(worker_id).await {
						tracing::error!(?err, "unhandled update ping error");
					}

					if let Err(err) = db.clear_expired_leases(worker_id).await {
						tracing::error!(?err, "unhandled gc error");
					}
				}
			}
			.instrument(tracing::info_span!("worker_gc_task")),
		)
	}

	fn publish_metrics(&self) -> JoinHandle<()> {
		let db = self.db.clone();
		let worker_id = self.worker_id;

		tokio::spawn(
			async move {
				let mut metrics_interval = tokio::time::interval(METRICS_INTERVAL);
				metrics_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

				loop {
					metrics_interval.tick().await;

					if let Err(err) = db.publish_metrics(worker_id).await {
						tracing::error!(?err, "unhandled metrics error");
					}
				}
			}
			.instrument(tracing::info_span!("worker_metrics_task")),
		)
	}

	#[tracing::instrument(skip_all)]
	async fn shutdown(mut self, mut term_signal: TermSignal) {
		let shutdown_duration = self.config.runtime.worker.shutdown_duration();

		tracing::info!(
			duration=?shutdown_duration,
			remaining_workflows=?self.running_workflows.len(),
			"starting worker shutdown"
		);

		if let Err(err) = self.db.mark_worker_inactive(self.worker_id).await {
			tracing::error!(?err, worker_id=?self.worker_id, "failed to mark worker as inactive");
		}

		// Send stop signal to all running workflows
		for (workflow_id, wf) in &self.running_workflows {
			if wf.stop.send(()).is_err() {
				tracing::debug!(
					?workflow_id,
					"stop channel closed, workflow likely already stopped"
				);
			}
		}

		// Collect all workflow tasks
		let mut wf_futs = self
			.running_workflows
			.iter_mut()
			.map(|(_, wf)| &mut wf.handle)
			.collect::<FuturesUnordered<_>>();

		let mut progress_interval = tokio::time::interval(SHUTDOWN_PROGRESS_INTERVAL);
		progress_interval.tick().await;

		let shutdown_start = Instant::now();
		loop {
			// Future will resolve once all workflow tasks complete
			let join_fut = async { while let Some(_) = wf_futs.next().await {} };

			tokio::select! {
				_ = join_fut => {
					break;
				}
				_ = progress_interval.tick() => {
					tracing::info!(remaining_workflows=%wf_futs.len(), "worker still shutting down");
				}
				abort = term_signal.recv() => {
					if abort {
						tracing::warn!("aborting worker shutdown");
						break;
					}
				}
				_ = tokio::time::sleep(shutdown_duration.saturating_sub(shutdown_start.elapsed())) => {
					tracing::warn!("worker shutdown timed out");
					break;
				}
			}
		}

		metrics::WORKER_WORKFLOW_ACTIVE.reset();

		let remaining_workflows = wf_futs.into_iter().count();
		if remaining_workflows == 0 {
			tracing::info!("all workflows evicted");
		} else {
			tracing::warn!(?remaining_workflows, "not all workflows evicted");
		}

		tracing::info!("worker shutdown complete");
	}
}

struct WorkflowHandle {
	name: String,
	stop: watch::Sender<()>,
	handle: JoinHandle<()>,
}
