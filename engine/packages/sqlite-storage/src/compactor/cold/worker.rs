use std::{ops::Deref, sync::Arc, time::{Duration, Instant}};

use anyhow::{Context, Result};
use gas::prelude::{Database, Id, StandaloneCtx, db};
use rivet_runtime::TermSignal;
use tokio::{
	sync::{Semaphore, watch},
	task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use universalpubsub::NextOutput;

use crate::{
	cold_tier::{ColdTier, DisabledColdTier},
	compactor::{
		SqliteColdCompactPayload, SqliteColdCompactSubject, decode_cold_compact_payload,
		publish::Ups,
	},
	pump::types::ActorBranchId,
};

use super::{lease, phase_a};

const COLD_COMPACTOR_QUEUE_GROUP: &str = "cold_compactor";

#[derive(Clone, Debug)]
pub struct ColdCompactorConfig {
	pub lease_ttl_ms: u64,
	pub lease_renew_interval_ms: u64,
	pub lease_margin_ms: u64,
	pub cold_compact_delta_threshold: u32,
	pub phase_a_read_timeout_ms: u64,
	pub max_concurrent_workers: u32,
	pub ups_subject: String,
}

impl Default for ColdCompactorConfig {
	fn default() -> Self {
		Self {
			lease_ttl_ms: 30_000,
			lease_renew_interval_ms: 10_000,
			lease_margin_ms: 5_000,
			cold_compact_delta_threshold: 1024,
			phase_a_read_timeout_ms: 5_000,
			max_concurrent_workers: 64,
			ups_subject: SqliteColdCompactSubject.to_string(),
		}
	}
}

#[tracing::instrument(skip_all)]
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	cold_config: ColdCompactorConfig,
) -> Result<()> {
	let node_id = pools.node_id();
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"sqlite_cold_compactor",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	run_with_node_id(
		Arc::new(ctx.udb()?.deref().clone()),
		ctx.ups()?,
		TermSignal::get(),
		default_cold_tier(),
		cold_config,
		node_id,
	)
	.await
}

#[tracing::instrument(skip_all)]
#[allow(dead_code)]
pub(crate) async fn run(
	udb: Arc<universaldb::Database>,
	ups: Ups,
	term_signal: TermSignal,
	cold_config: ColdCompactorConfig,
) -> Result<()> {
	run_with_node_id(
		udb,
		ups,
		term_signal,
		default_cold_tier(),
		cold_config,
		rivet_pools::NodeId::new(),
	)
	.await
}

async fn run_with_node_id(
	udb: Arc<universaldb::Database>,
	ups: Ups,
	mut term_signal: TermSignal,
	cold_tier: Arc<dyn ColdTier>,
	cold_config: ColdCompactorConfig,
	holder_id: rivet_pools::NodeId,
) -> Result<()> {
	let mut sub = ups
		.queue_subscribe(cold_config.ups_subject.as_str(), COLD_COMPACTOR_QUEUE_GROUP)
		.await?;
	let max_workers = usize::try_from(cold_config.max_concurrent_workers)
		.context("sqlite cold compactor max_concurrent_workers exceeded usize")?
		.max(1);
	let semaphore = Arc::new(Semaphore::new(max_workers));
	let shutdown = CancellationToken::new();
	let mut workers = JoinSet::new();

	let loop_result = loop {
		tokio::select! {
			msg = sub.next() => {
				match msg? {
					NextOutput::Message(msg) => {
						let payload = match decode_cold_compact_payload(&msg.payload) {
							Ok(payload) => payload,
							Err(err) => {
								tracing::warn!(?err, "received invalid sqlite cold compact trigger");
								continue;
							}
						};
						let udb = Arc::clone(&udb);
						let shutdown = shutdown.child_token();
						let semaphore = Arc::clone(&semaphore);
						let cold_tier = Arc::clone(&cold_tier);
						let cold_config = cold_config.clone();

						workers.spawn(async move {
							let Ok(_permit) = semaphore.acquire_owned().await else {
								return;
							};
							if let Err(err) = handle_trigger(
								udb,
								payload,
								cold_tier,
								cold_config,
								holder_id,
								shutdown,
							).await {
								tracing::warn!(?err, "sqlite cold compactor trigger failed");
							}
						});
					}
					NextOutput::Unsubscribed => break Err(anyhow::anyhow!("sqlite cold compactor sub unsubscribed")),
				}
			}
			_ = term_signal.recv() => break Ok(()),
			Some(join_result) = workers.join_next(), if !workers.is_empty() => {
				if let Err(err) = join_result {
					tracing::warn!(?err, "sqlite cold compactor worker task panicked");
				}
			}
		}
	};

	shutdown.cancel();
	while let Some(join_result) = workers.join_next().await {
		if let Err(err) = join_result {
			tracing::warn!(?err, "sqlite cold compactor worker task panicked during shutdown");
		}
	}

	loop_result
}

async fn handle_trigger(
	udb: Arc<universaldb::Database>,
	payload: SqliteColdCompactPayload,
	cold_tier: Arc<dyn ColdTier>,
	cold_config: ColdCompactorConfig,
	holder_id: rivet_pools::NodeId,
	shutdown: CancellationToken,
) -> Result<()> {
	if shutdown.is_cancelled() {
		return Ok(());
	}

	let branch_id = payload_branch_id(&payload);
	let now_ms = now_ms()?;
	let lease_ttl_ms = cold_config.lease_ttl_ms;
	let take_outcome = udb
		.run({
			move |tx| async move {
				lease::take(&tx, branch_id, holder_id, lease_ttl_ms, now_ms).await
			}
		})
		.await?;

	if matches!(take_outcome, lease::ColdTakeOutcome::Skip) {
		return Ok(());
	}

	let lease_started_at = Instant::now();
	let cancel_token = shutdown.child_token();
	let initial_deadline = tokio::time::Instant::now() + lease_deadline_after(&cold_config)?;
	let (deadline_tx, deadline_rx) = watch::channel(initial_deadline);
	let renewal_handle = spawn_renewal_task(
		Arc::clone(&udb),
		branch_id,
		holder_id,
		cold_config.clone(),
		cancel_token.clone(),
		deadline_tx,
	);
	let deadline_handle = spawn_deadline_task(deadline_rx, cancel_token.clone());

	let result = run_scaffold_pass(
		udb.as_ref(),
		payload,
		cold_tier,
		&cold_config,
		cancel_token.clone(),
		holder_id,
	)
	.await;

	cancel_token.cancel();
	renewal_handle.abort();
	deadline_handle.abort();

	let release_result = udb
		.run(move |tx| async move { lease::release(&tx, branch_id, holder_id).await })
		.await;

	if let Err(err) = release_result {
		tracing::warn!(?err, ?branch_id, "failed to release sqlite cold compactor lease");
	}

	tracing::debug!(
		?branch_id,
		node_id = %holder_id,
		held_seconds = lease_started_at.elapsed().as_secs_f64(),
		"sqlite cold compactor lease released"
	);

	result
}

async fn run_scaffold_pass(
	udb: &universaldb::Database,
	payload: SqliteColdCompactPayload,
	cold_tier: Arc<dyn ColdTier>,
	cold_config: &ColdCompactorConfig,
	cancel_token: CancellationToken,
	holder_id: rivet_pools::NodeId,
) -> Result<()> {
	if cancel_token.is_cancelled() {
		anyhow::bail!("sqlite cold compaction cancelled");
	}

	tracing::debug!(
		?payload,
		node_id = %holder_id,
		cold_compact_delta_threshold = cold_config.cold_compact_delta_threshold,
		"sqlite cold compactor pass scaffold received trigger"
	);

	let plan = phase_a::run(
		udb,
		cold_tier,
		payload,
		cold_config,
		cancel_token,
		now_ms()?,
	)
	.await?;
	tracing::debug!(
		branch_id = ?plan.branch_id,
		pass_uuid = %plan.pass_uuid,
		planned_object_keys = plan.marker.planned_object_keys.len(),
		shard_versions = plan.shard_versions.len(),
		delta_chunks = plan.delta_chunks.len(),
		commit_rows = plan.commit_rows.len(),
		vtx_rows = plan.vtx_rows.len(),
		"sqlite cold compactor phase A planned pass"
	);

	Ok(())
}

fn payload_branch_id(payload: &SqliteColdCompactPayload) -> ActorBranchId {
	match payload {
		SqliteColdCompactPayload::CreatePinnedBookmark {
			actor_branch_id, ..
		}
		| SqliteColdCompactPayload::DeletePinnedBookmark {
			actor_branch_id, ..
		} => *actor_branch_id,
	}
}

fn spawn_renewal_task(
	udb: Arc<universaldb::Database>,
	branch_id: ActorBranchId,
	holder_id: rivet_pools::NodeId,
	cold_config: ColdCompactorConfig,
	cancel_token: CancellationToken,
	deadline_tx: watch::Sender<tokio::time::Instant>,
) -> tokio::task::JoinHandle<()> {
	tokio::spawn(async move {
		let mut interval =
			tokio::time::interval(Duration::from_millis(cold_config.lease_renew_interval_ms));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		interval.tick().await;

		loop {
			tokio::select! {
				_ = cancel_token.cancelled() => return,
				_ = interval.tick() => {
					if cancel_token.is_cancelled() {
						return;
					}
					let now_ms = match now_ms() {
						Ok(now_ms) => now_ms,
						Err(err) => {
							tracing::warn!(?err, ?branch_id, "failed to compute sqlite cold compactor renewal timestamp");
							cancel_token.cancel();
							return;
						}
					};
					let lease_ttl_ms = cold_config.lease_ttl_ms;
					let renew_result = udb
						.run(move |tx| async move {
							lease::renew(&tx, branch_id, holder_id, lease_ttl_ms, now_ms).await
						})
						.await;

					match renew_result {
						Ok(lease::ColdRenewOutcome::Renewed) => {
							match lease_deadline_after(&cold_config) {
								Ok(deadline_after) => {
									let _ = deadline_tx.send(tokio::time::Instant::now() + deadline_after);
								}
								Err(err) => {
									tracing::warn!(?err, ?branch_id, "failed to compute sqlite cold compactor lease deadline");
									cancel_token.cancel();
									return;
								}
							}
						}
						Ok(outcome) => {
							tracing::warn!(?outcome, ?branch_id, "sqlite cold compactor lease renewal stopped compaction");
							cancel_token.cancel();
							return;
						}
						Err(err) => {
							tracing::warn!(?err, ?branch_id, "sqlite cold compactor lease renewal failed");
							cancel_token.cancel();
							return;
						}
					}
				}
			}
		}
	})
}

fn spawn_deadline_task(
	mut deadline_rx: watch::Receiver<tokio::time::Instant>,
	cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
	tokio::spawn(async move {
		loop {
			let deadline = *deadline_rx.borrow();

			tokio::select! {
				_ = cancel_token.cancelled() => return,
				_ = tokio::time::sleep_until(deadline) => {
					tracing::warn!("sqlite cold compactor lease local deadline elapsed");
					cancel_token.cancel();
					return;
				}
				changed = deadline_rx.changed() => {
					if changed.is_err() {
						return;
					}
				}
			}
		}
	})
}

fn lease_deadline_after(cold_config: &ColdCompactorConfig) -> Result<Duration> {
	let ttl = Duration::from_millis(cold_config.lease_ttl_ms);
	let margin = Duration::from_millis(cold_config.lease_margin_ms);
	ttl
		.checked_sub(margin)
		.context("sqlite cold compactor lease margin must be less than ttl")
}

fn now_ms() -> Result<i64> {
	let elapsed = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite cold compactor timestamp exceeded i64")
}

fn default_cold_tier() -> Arc<dyn ColdTier> {
	Arc::new(DisabledColdTier)
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use super::*;

	pub async fn handle_payload_once(
		udb: Arc<universaldb::Database>,
		payload: SqliteColdCompactPayload,
		cold_config: ColdCompactorConfig,
		cancel_token: CancellationToken,
	) -> Result<()> {
		handle_payload_once_with_cold_tier(
			udb,
			payload,
			cold_config,
			cancel_token,
			default_cold_tier(),
		)
		.await
	}

	pub async fn handle_payload_once_with_cold_tier(
		udb: Arc<universaldb::Database>,
		payload: SqliteColdCompactPayload,
		cold_config: ColdCompactorConfig,
		cancel_token: CancellationToken,
		cold_tier: Arc<dyn ColdTier>,
	) -> Result<()> {
		handle_trigger(
			udb,
			payload,
			cold_tier,
			cold_config,
			rivet_pools::NodeId::new(),
			cancel_token,
		)
		.await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_config_matches_spec() {
		let config = ColdCompactorConfig::default();

		assert_eq!(config.lease_ttl_ms, 30_000);
		assert_eq!(config.lease_renew_interval_ms, 10_000);
		assert_eq!(config.lease_margin_ms, 5_000);
		assert_eq!(config.cold_compact_delta_threshold, 1024);
		assert_eq!(config.phase_a_read_timeout_ms, 5_000);
		assert_eq!(config.max_concurrent_workers, 64);
		assert_eq!(config.ups_subject, "sqlite.cold_compact");
	}
}
