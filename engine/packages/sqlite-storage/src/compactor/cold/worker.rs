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
		metrics,
		publish::Ups,
	},
	gc,
	pump::types::DatabaseBranchId,
};

use super::{lease, phase_a, phase_b, phase_c, phase_d, phase_warmup};

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

	let Some(branch_id) = payload_branch_id(&payload) else {
		return handle_namespace_warmup(payload);
	};
	let now_ms = now_ms()?;
	let lease_ttl_ms = cold_config.lease_ttl_ms;
	let take_outcome = udb
		.run({
			move |tx| async move {
				lease::take(&tx, branch_id, holder_id, lease_ttl_ms, now_ms).await
			}
		})
		.await?;
	let holder_label = holder_id.to_string();
	let take_outcome_label = match take_outcome {
		lease::ColdTakeOutcome::Acquired => "acquired",
		lease::ColdTakeOutcome::Skip => "skipped",
	};
	metrics::SQLITE_COLD_LEASE_TAKE_TOTAL
		.with_label_values(&[holder_label.as_str(), take_outcome_label])
		.inc();

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
		Arc::clone(&cold_tier),
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

	if let SqliteColdCompactPayload::ForkWarmup {
		source_database_branch_id,
		target_database_branch_id,
		at_versionstamp,
	} = payload
	{
		let output = phase_warmup::run_database(
			Arc::clone(&cold_tier),
			source_database_branch_id,
			target_database_branch_id,
			at_versionstamp,
			cancel_token,
			now_ms()?,
		)
		.await?;
		tracing::debug!(
			source_branch_id = ?source_database_branch_id,
			target_branch_id = ?target_database_branch_id,
			copied_layers = output.copied_layers,
			source_chunks_read = output.source_chunks_read,
			"sqlite cold compactor fork warmup finished"
		);

		return Ok(());
	}

	let payload_for_failure = payload.clone();
	let holder_label = holder_id.to_string();
	let plan = {
		let _timer = metrics::SQLITE_COLD_PASS_DURATION
			.with_label_values(&[holder_label.as_str(), "A"])
			.start_timer();
		phase_a::run(
			udb,
			Arc::clone(&cold_tier),
			payload,
			cold_config,
			cancel_token.clone(),
			now_ms()?,
		)
		.await?
	};
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
	let phase_b_result = {
		let _timer = metrics::SQLITE_COLD_PASS_DURATION
			.with_label_values(&[holder_label.as_str(), "B"])
			.start_timer();
		phase_b::run(
			Arc::clone(&cold_tier),
			&plan,
			cancel_token.clone(),
			now_ms()?,
		)
		.await
	};
	let phase_b_output = match phase_b_result {
		Ok(output) => output,
		Err(err) => {
			match phase_c::mark_payload_pins_failed(udb, &payload_for_failure, now_ms()?).await {
				Ok(failed_pins) => {
					tracing::warn!(
						branch_id = ?plan.branch_id,
						pass_uuid = %plan.pass_uuid,
						failed_pins,
						?err,
						"sqlite cold compactor phase B failed and marked pins failed"
					);
				}
				Err(mark_err) => {
					tracing::warn!(
						branch_id = ?plan.branch_id,
						pass_uuid = %plan.pass_uuid,
						?err,
						?mark_err,
						"sqlite cold compactor phase B failed and failed to mark pins failed"
					);
				}
			}
			return Err(err);
		}
	};
	metrics::SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL
		.with_label_values(&[holder_label.as_str(), "image"])
		.inc_by(plan.shard_versions.len() as u64);
	metrics::SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL
		.with_label_values(&[holder_label.as_str(), "pin"])
		.inc_by(phase_b_output.uploaded_pins.len() as u64);
	let delta_layers = phase_b_output
		.layer_count
		.saturating_sub(plan.shard_versions.len())
		.saturating_sub(phase_b_output.uploaded_pins.len());
	metrics::SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL
		.with_label_values(&[holder_label.as_str(), "delta"])
		.inc_by(delta_layers as u64);
	metrics::SQLITE_COLD_PASS_BYTES_UPLOADED_TOTAL
		.with_label_values(&[holder_label.as_str()])
		.inc_by(phase_b_output.bytes_uploaded);
	metrics::SQLITE_PENDING_MARKER_ORPHAN_CLEANED_TOTAL
		.with_label_values(&[holder_label.as_str()])
		.inc_by(phase_b_output.stale_markers_cleaned as u64);
	tracing::debug!(
		branch_id = ?plan.branch_id,
		pass_uuid = %plan.pass_uuid,
		layers = phase_b_output.layer_count,
		bookmarks = phase_b_output.bookmark_count,
		stale_markers_cleaned = phase_b_output.stale_markers_cleaned,
		"sqlite cold compactor phase B uploaded pass"
	);
	let phase_c_output = {
		let _timer = metrics::SQLITE_COLD_PASS_DURATION
			.with_label_values(&[holder_label.as_str(), "C"])
			.start_timer();
		phase_c::run(
			udb,
			&plan,
			&phase_b_output,
			cancel_token.clone(),
			now_ms()?,
		)
		.await?
	};
	tracing::debug!(
		branch_id = ?plan.branch_id,
		pass_uuid = %plan.pass_uuid,
		cold_drained_txid = phase_c_output.cold_drained_txid,
		ready_pins = phase_c_output.ready_pins,
		"sqlite cold compactor phase C committed pass"
	);
	let sweep_output = phase_d::run(
		udb,
		Arc::clone(&cold_tier),
		plan.branch_id,
		cancel_token,
	)
	.await?;
	tracing::debug!(
		branch_id = ?plan.branch_id,
		pass_uuid = %plan.pass_uuid,
		removed_chunks = sweep_output.removed_chunks,
		removed_layers = sweep_output.removed_layers,
		deleted_objects = sweep_output.deleted_objects,
		"sqlite cold compactor follow-up sweep finished"
	);
	let hot_gc_output = gc::sweep_branch_hot_history(udb, plan.branch_id).await?;
	if let Some(hot_gc_output) = hot_gc_output {
		tracing::debug!(
			branch_id = ?plan.branch_id,
			pass_uuid = %plan.pass_uuid,
			gc_pin = ?hot_gc_output.gc_pin,
			txid_floor = ?hot_gc_output.txid_floor,
			commits_deleted = hot_gc_output.commits_deleted,
			vtx_deleted = hot_gc_output.vtx_deleted,
			delta_chunks_deleted = hot_gc_output.delta_chunks_deleted,
			"sqlite cold compactor hot history GC finished"
		);
	}

	Ok(())
}

fn payload_branch_id(payload: &SqliteColdCompactPayload) -> Option<DatabaseBranchId> {
	match payload {
		SqliteColdCompactPayload::CreatePinnedBookmark {
			database_branch_id, ..
		}
		| SqliteColdCompactPayload::DeletePinnedBookmark {
			database_branch_id, ..
		} => Some(*database_branch_id),
		SqliteColdCompactPayload::ForkWarmup {
			target_database_branch_id,
			..
		} => Some(*target_database_branch_id),
		SqliteColdCompactPayload::NamespaceForkWarmup { .. } => None,
	}
}

fn handle_namespace_warmup(payload: SqliteColdCompactPayload) -> Result<()> {
	let SqliteColdCompactPayload::NamespaceForkWarmup {
		source_namespace_branch_id,
		target_namespace_branch_id,
		at_versionstamp,
	} = payload
	else {
		return Ok(());
	};

	tracing::debug!(
		?source_namespace_branch_id,
		?target_namespace_branch_id,
		?at_versionstamp,
		"sqlite cold compactor received namespace fork warmup"
	);

	Ok(())
}

fn spawn_renewal_task(
	udb: Arc<universaldb::Database>,
	branch_id: DatabaseBranchId,
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
