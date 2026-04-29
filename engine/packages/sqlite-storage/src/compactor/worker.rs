use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use gas::prelude::{Database, Id, StandaloneCtx, db};
use rivet_runtime::TermSignal;
use tokio::{
	sync::{Semaphore, watch},
	task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use universalpubsub::NextOutput;

use crate::{
	admin::{self, SqliteOp, SqliteOpRequest},
	pump::{
		keys,
		quota,
		types::{
			ForkMarker, RestoreMarker, decode_fork_marker, decode_restore_marker,
		},
	},
};

use super::{
	SqliteCompactPayload, SqliteCompactSubject, SqliteOpSubject, TakeOutcome,
	compact::{CheckpointConfig, compact_default_batch_with_checkpoint_config},
	decode_compact_payload, lease, metrics, orphan, publish::Ups,
};

const COMPACTOR_QUEUE_GROUP: &str = "compactor";
const ORPHAN_SCAN_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub struct CompactorConfig {
	pub lease_ttl_ms: u64,
	pub lease_renew_interval_ms: u64,
	pub lease_margin_ms: u64,
	pub compaction_delta_threshold: u32,
	pub batch_size_deltas: u32,
	pub max_concurrent_workers: u32,
	pub pitr_enabled: bool,
	pub max_concurrent_checkpoints: u32,
	pub ups_subject: String,
	#[cfg(debug_assertions)]
	pub quota_validate_every: u32,
}

impl Default for CompactorConfig {
	fn default() -> Self {
		Self {
			lease_ttl_ms: 30_000,
			lease_renew_interval_ms: 10_000,
			lease_margin_ms: 5_000,
			compaction_delta_threshold: 32,
			batch_size_deltas: 32,
			max_concurrent_workers: 64,
			pitr_enabled: false,
			max_concurrent_checkpoints: 16,
			ups_subject: SqliteCompactSubject.to_string(),
			#[cfg(debug_assertions)]
			quota_validate_every: 16,
		}
	}
}

#[tracing::instrument(skip_all)]
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	compactor_config: CompactorConfig,
) -> Result<()> {
	let node_id = pools.node_id();
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"sqlite_compactor",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	run_with_node_id(
		Arc::new(ctx.udb()?.deref().clone()),
		ctx.ups()?,
		TermSignal::get(),
		compactor_config,
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
	compactor_config: CompactorConfig,
) -> Result<()> {
	run_with_node_id(
		udb,
		ups,
		term_signal,
		compactor_config,
		rivet_pools::NodeId::new(),
	)
	.await
}

async fn run_with_node_id(
	udb: Arc<universaldb::Database>,
	ups: Ups,
	mut term_signal: TermSignal,
	compactor_config: CompactorConfig,
	holder_id: rivet_pools::NodeId,
) -> Result<()> {
	let mut compact_sub = ups
		.queue_subscribe(compactor_config.ups_subject.as_str(), COMPACTOR_QUEUE_GROUP)
		.await?;
	let mut op_sub = ups
		.queue_subscribe(SqliteOpSubject, COMPACTOR_QUEUE_GROUP)
		.await?;
	let max_workers = usize::try_from(compactor_config.max_concurrent_workers)
		.context("sqlite compactor max_concurrent_workers exceeded usize")?
		.max(1);
	let semaphore = Arc::new(Semaphore::new(max_workers));
	let max_checkpoints = usize::try_from(compactor_config.max_concurrent_checkpoints)
		.context("sqlite compactor max_concurrent_checkpoints exceeded usize")?
		.max(1);
	let checkpoint_semaphore = Arc::new(Semaphore::new(max_checkpoints));
	let shutdown = CancellationToken::new();
	let mut workers = JoinSet::new();
	let mut orphan_interval = tokio::time::interval(ORPHAN_SCAN_INTERVAL);
	orphan_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	#[cfg(debug_assertions)]
	test_hooks::notify_run_ready();
	#[cfg(debug_assertions)]
	let quota_validate_counts = Arc::new(scc::HashMap::new());

	let loop_result = loop {
		tokio::select! {
			msg = compact_sub.next() => {
				match msg? {
					NextOutput::Message(msg) => {
						let payload = match decode_compact_payload(&msg.payload) {
							Ok(payload) => payload,
							Err(err) => {
								tracing::warn!(?err, "received invalid sqlite compact trigger");
								continue;
							}
						};
						let udb = Arc::clone(&udb);
						let shutdown = shutdown.child_token();
						let semaphore = Arc::clone(&semaphore);
						let checkpoint_semaphore = Arc::clone(&checkpoint_semaphore);
						let compactor_config = compactor_config.clone();
						#[cfg(debug_assertions)]
						let quota_validate_counts = Arc::clone(&quota_validate_counts);

						workers.spawn(async move {
							let Ok(_permit) = semaphore.acquire_owned().await else {
								return;
							};
							if let Err(err) = handle_trigger(
								udb,
								payload,
								compactor_config,
								holder_id,
								shutdown,
								checkpoint_semaphore,
								#[cfg(debug_assertions)]
								quota_validate_counts,
							).await {
								tracing::warn!(?err, "sqlite compactor trigger failed");
							}
						});
					}
					NextOutput::Unsubscribed => break Err(anyhow::anyhow!("sqlite compactor compact sub unsubscribed")),
				}
			}
			msg = op_sub.next() => {
				match msg? {
					NextOutput::Message(msg) => {
						let request = match admin::decode_sqlite_op_request(&msg.payload) {
							Ok(request) => request,
							Err(err) => {
								tracing::warn!(?err, "received invalid sqlite admin op request");
								continue;
							}
						};
						let udb = Arc::clone(&udb);
						let semaphore = Arc::clone(&semaphore);

						workers.spawn(async move {
							let Ok(_permit) = semaphore.acquire_owned().await else {
								return;
							};
							if let Err(err) = handle_admin_op(udb, request, holder_id).await {
								tracing::warn!(?err, "sqlite admin op failed");
							}
						});
					}
					NextOutput::Unsubscribed => break Err(anyhow::anyhow!("sqlite compactor op sub unsubscribed")),
				}
			}
			_ = orphan_interval.tick() => {
				let now_ms = now_ms()?;
				if let Err(err) = orphan::scan_for_orphans(Arc::clone(&udb), now_ms).await {
					tracing::warn!(?err, "sqlite admin orphan scan failed");
				}
			}
			_ = term_signal.recv() => break Ok(()),
			Some(join_result) = workers.join_next(), if !workers.is_empty() => {
				if let Err(err) = join_result {
					tracing::warn!(?err, "sqlite compactor worker task panicked");
				}
			}
		}
	};

	shutdown.cancel();
	while let Some(join_result) = workers.join_next().await {
		if let Err(err) = join_result {
			tracing::warn!(?err, "sqlite compactor worker task panicked during shutdown");
		}
	}

	loop_result
}

async fn handle_admin_op(
	udb: Arc<universaldb::Database>,
	request: SqliteOpRequest,
	holder_id: rivet_pools::NodeId,
) -> Result<()> {
	let op_label = admin_op_label(&request.op);
	let _in_flight = AdminOpInFlightGuard::new(op_label);
	let record = match admin::start_work(Arc::clone(&udb), request.request_id, holder_id).await? {
		Some(record) => record,
		None => {
			tracing::debug!(
				operation_id = %request.request_id,
				"sqlite admin op has no runnable record"
			);
			return Ok(());
		}
	};
	validate_admin_request_matches_record(&request, &record)?;

	match request.op.clone() {
		SqliteOp::Restore {
			actor_id,
			target,
			mode,
		} => {
			handle_restore(udb, request, record, actor_id, target, mode, holder_id).await
		}
		SqliteOp::Fork {
			src_actor_id,
			target,
			mode,
			dst,
		} => {
			handle_fork(udb, request, record, src_actor_id, target, mode, dst, holder_id).await
		}
		SqliteOp::DescribeRetention { actor_id } => {
			handle_describe_retention(udb, request, record, actor_id, holder_id).await
		}
		SqliteOp::SetRetention { actor_id, config } => {
			handle_set_retention(udb, request, record, actor_id, config, holder_id).await
		}
		SqliteOp::ClearRefcount {
			actor_id,
			kind,
			txid,
		} => handle_clear_refcount(udb, request, record, actor_id, kind, txid, holder_id).await,
	}
}

async fn handle_restore(
	udb: Arc<universaldb::Database>,
	request: SqliteOpRequest,
	record: admin::AdminOpRecord,
	actor_id: String,
	target: admin::RestoreTarget,
	mode: admin::RestoreMode,
	holder_id: rivet_pools::NodeId,
) -> Result<()> {
	let resume = restore_resume_state(Arc::clone(&udb), &actor_id, request.request_id).await?;
	#[cfg(debug_assertions)]
	if test_hooks::maybe_handle_admin_op(test_hooks::AdminOpHookEvent {
		request,
		record,
		resume: resume.clone(),
	})
	.await?
	{
		return Ok(());
	}

	let _ = (udb, actor_id, target, mode, holder_id, resume);
	unimplemented!("sqlite restore admin op handler lands in US-034");
}

async fn handle_fork(
	udb: Arc<universaldb::Database>,
	request: SqliteOpRequest,
	record: admin::AdminOpRecord,
	src_actor_id: String,
	target: admin::RestoreTarget,
	mode: admin::ForkMode,
	dst: admin::ForkDstSpec,
	holder_id: rivet_pools::NodeId,
) -> Result<()> {
	let marker_actor_id = fork_marker_actor_id(&src_actor_id, &dst);
	let resume = fork_resume_state(Arc::clone(&udb), &marker_actor_id, request.request_id).await?;
	#[cfg(debug_assertions)]
	if test_hooks::maybe_handle_admin_op(test_hooks::AdminOpHookEvent {
		request,
		record,
		resume: resume.clone(),
	})
	.await?
	{
		return Ok(());
	}

	let _ = (udb, src_actor_id, target, mode, dst, holder_id, resume);
	unimplemented!("sqlite fork admin op handler lands in US-035");
}

async fn handle_describe_retention(
	_udb: Arc<universaldb::Database>,
	request: SqliteOpRequest,
	record: admin::AdminOpRecord,
	actor_id: String,
	_holder_id: rivet_pools::NodeId,
) -> Result<()> {
	#[cfg(debug_assertions)]
	if test_hooks::maybe_handle_admin_op(test_hooks::AdminOpHookEvent {
		request,
		record,
		resume: None,
	})
	.await?
	{
		return Ok(());
	}

	let _ = actor_id;
	unimplemented!("sqlite describe-retention admin op handler lands in US-036");
}

async fn handle_set_retention(
	_udb: Arc<universaldb::Database>,
	request: SqliteOpRequest,
	record: admin::AdminOpRecord,
	actor_id: String,
	config: crate::pump::types::RetentionConfig,
	_holder_id: rivet_pools::NodeId,
) -> Result<()> {
	#[cfg(debug_assertions)]
	if test_hooks::maybe_handle_admin_op(test_hooks::AdminOpHookEvent {
		request,
		record,
		resume: None,
	})
	.await?
	{
		return Ok(());
	}

	let _ = (actor_id, config);
	unimplemented!("sqlite set-retention admin op handler lands in US-036");
}

async fn handle_clear_refcount(
	_udb: Arc<universaldb::Database>,
	request: SqliteOpRequest,
	record: admin::AdminOpRecord,
	actor_id: String,
	kind: admin::RefcountKind,
	txid: u64,
	_holder_id: rivet_pools::NodeId,
) -> Result<()> {
	#[cfg(debug_assertions)]
	if test_hooks::maybe_handle_admin_op(test_hooks::AdminOpHookEvent {
		request,
		record,
		resume: None,
	})
	.await?
	{
		return Ok(());
	}

	let _ = (actor_id, kind, txid);
	unimplemented!("sqlite clear-refcount admin op handler lands in US-036");
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdminResumeState {
	RestoreMatching {
		marker: RestoreMarker,
	},
	RestoreTakeover {
		previous_op_id: uuid::Uuid,
		marker: RestoreMarker,
	},
	ForkMatching {
		marker: ForkMarker,
	},
	ForkTakeover {
		previous_op_id: uuid::Uuid,
		marker: ForkMarker,
	},
}

async fn restore_resume_state(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	op_id: uuid::Uuid,
) -> Result<Option<AdminResumeState>> {
	let actor_id = actor_id.to_string();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let Some(marker) = read_restore_marker(&tx, &actor_id).await? else {
				return Ok(None);
			};
			if marker.op_id == op_id {
				return Ok(Some(AdminResumeState::RestoreMatching { marker }));
			}
			ensure_marker_holder_is_stale(&tx, &actor_id, marker.holder_id).await?;
			Ok(Some(AdminResumeState::RestoreTakeover {
				previous_op_id: marker.op_id,
				marker,
			}))
		}
	})
	.await
}

async fn fork_resume_state(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	op_id: uuid::Uuid,
) -> Result<Option<AdminResumeState>> {
	let actor_id = actor_id.to_string();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let Some(marker) = read_fork_marker(&tx, &actor_id).await? else {
				return Ok(None);
			};
			if marker.op_id == op_id {
				return Ok(Some(AdminResumeState::ForkMatching { marker }));
			}
			ensure_marker_holder_is_stale(&tx, &actor_id, marker.holder_id).await?;
			Ok(Some(AdminResumeState::ForkTakeover {
				previous_op_id: marker.op_id,
				marker,
			}))
		}
	})
	.await
}

async fn read_restore_marker(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<Option<RestoreMarker>> {
	Ok(match tx
		.informal()
		.get(
			&keys::meta_restore_in_progress_key(actor_id),
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
	{
		Some(value) => Some(decode_restore_marker(&value)?),
		None => None,
	})
}

async fn read_fork_marker(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<Option<ForkMarker>> {
	Ok(match tx
		.informal()
		.get(
			&keys::meta_fork_in_progress_key(actor_id),
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
	{
		Some(value) => Some(decode_fork_marker(&value)?),
		None => None,
	})
}

async fn ensure_marker_holder_is_stale(
	tx: &universaldb::Transaction,
	actor_id: &str,
	marker_holder: rivet_pools::NodeId,
) -> Result<()> {
	let Some(lease_value) = tx
		.informal()
		.get(
			&keys::meta_compactor_lease_key(actor_id),
			universaldb::utils::IsolationLevel::Serializable,
		)
		.await?
	else {
		return Ok(());
	};
	let lease = lease::decode_lease(&lease_value)?;
	let now_ms = now_ms()?;
	if lease.holder_id == marker_holder && lease.expires_at_ms > now_ms {
		bail!("sqlite admin marker holder still has a live lease");
	}
	Ok(())
}

fn validate_admin_request_matches_record(
	request: &SqliteOpRequest,
	record: &admin::AdminOpRecord,
) -> Result<()> {
	if request.request_id != record.operation_id {
		bail!("sqlite admin op request id did not match persisted record");
	}
	if admin_op_kind(&request.op) != record.op_kind {
		bail!("sqlite admin op kind did not match persisted record");
	}
	Ok(())
}

fn admin_op_kind(op: &SqliteOp) -> admin::OpKind {
	match op {
		SqliteOp::Restore { .. } => admin::OpKind::Restore,
		SqliteOp::Fork { .. } => admin::OpKind::Fork,
		SqliteOp::DescribeRetention { .. } => admin::OpKind::DescribeRetention,
		SqliteOp::SetRetention { .. } => admin::OpKind::SetRetention,
		SqliteOp::ClearRefcount { .. } => admin::OpKind::ClearRefcount,
	}
}

fn admin_op_label(op: &SqliteOp) -> &'static str {
	match op {
		SqliteOp::Restore { .. } => "restore",
		SqliteOp::Fork { .. } => "fork",
		SqliteOp::DescribeRetention { .. } => "describe_retention",
		SqliteOp::SetRetention { .. } => "set_retention",
		SqliteOp::ClearRefcount { .. } => "clear_refcount",
	}
}

fn fork_marker_actor_id(src_actor_id: &str, dst: &admin::ForkDstSpec) -> String {
	match dst {
		admin::ForkDstSpec::Allocate { .. } => src_actor_id.to_string(),
		admin::ForkDstSpec::Existing { dst_actor_id } => dst_actor_id.clone(),
	}
}

struct AdminOpInFlightGuard {
	op: &'static str,
}

impl AdminOpInFlightGuard {
	fn new(op: &'static str) -> Self {
		metrics::SQLITE_ADMIN_OP_IN_FLIGHT
			.with_label_values(&[op])
			.inc();
		Self { op }
	}
}

impl Drop for AdminOpInFlightGuard {
	fn drop(&mut self) {
		metrics::SQLITE_ADMIN_OP_IN_FLIGHT
			.with_label_values(&[self.op])
			.dec();
	}
}

async fn handle_trigger(
	udb: Arc<universaldb::Database>,
	payload: SqliteCompactPayload,
	compactor_config: CompactorConfig,
	holder_id: rivet_pools::NodeId,
	shutdown: CancellationToken,
	checkpoint_semaphore: Arc<Semaphore>,
	#[cfg(debug_assertions)] quota_validate_counts: Arc<scc::HashMap<String, u32>>,
) -> Result<()> {
	if shutdown.is_cancelled() {
		return Ok(());
	}

	let actor_id = payload.actor_id.clone();
	let now_ms = now_ms()?;
	let node_id = holder_id.to_string();
	let take_result = udb
		.run({
			let actor_id = actor_id.clone();
			move |tx| {
				let actor_id = actor_id.clone();
				async move {
					lease::take(
						&tx,
						&actor_id,
						holder_id,
						compactor_config.lease_ttl_ms,
						now_ms,
					)
					.await
				}
			}
		})
		.await;

	match &take_result {
		Ok(TakeOutcome::Acquired) => metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL
			.with_label_values(&[node_id.as_str(), "acquired"])
			.inc(),
		Ok(TakeOutcome::Skip) => metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL
			.with_label_values(&[node_id.as_str(), "skipped"])
			.inc(),
		Err(_) => metrics::SQLITE_COMPACTOR_LEASE_TAKE_TOTAL
			.with_label_values(&[node_id.as_str(), "conflict"])
			.inc(),
	}
	let take_outcome = take_result?;

	if matches!(take_outcome, TakeOutcome::Skip) {
		return Ok(());
	}

	let lease_started_at = Instant::now();
	let cancel_token = shutdown.child_token();
	let initial_deadline = tokio::time::Instant::now() + lease_deadline_after(&compactor_config)?;
	let (deadline_tx, deadline_rx) = watch::channel(initial_deadline);
	let renewal_handle = spawn_renewal_task(
		Arc::clone(&udb),
		actor_id.clone(),
		holder_id,
		compactor_config.clone(),
		cancel_token.clone(),
		deadline_tx,
	);
	let deadline_handle = spawn_deadline_task(deadline_rx, cancel_token.clone());

	let result = async {
		let checkpoint_config = if compactor_config.pitr_enabled {
			load_checkpoint_config(
				Arc::clone(&udb),
				payload.namespace_id,
				payload.actor_name.clone(),
				compactor_config.lease_ttl_ms,
				Arc::clone(&checkpoint_semaphore),
			)
			.await?
		} else {
			None
		};
		compact_default_batch_with_checkpoint_config(
			Arc::clone(&udb),
			actor_id.clone(),
			compactor_config.batch_size_deltas,
			cancel_token.clone(),
			holder_id,
			checkpoint_config,
		)
		.await?;
		#[cfg(debug_assertions)]
		maybe_validate_quota(
			Arc::clone(&udb),
			actor_id.clone(),
			&compactor_config,
			&quota_validate_counts,
			holder_id,
		)
		.await?;
		emit_metering_rollup(Arc::clone(&udb), payload, holder_id).await
	}
	.await;

	cancel_token.cancel();
	renewal_handle.abort();
	deadline_handle.abort();

	let release_result = udb
		.run({
			let actor_id = actor_id.clone();
			move |tx| {
				let actor_id = actor_id.clone();
				async move { lease::release(&tx, &actor_id, holder_id).await }
			}
		})
		.await;

	if let Err(err) = release_result {
		tracing::warn!(?err, actor_id = %actor_id, "failed to release sqlite compactor lease");
	}
	metrics::SQLITE_COMPACTOR_LEASE_HELD_SECONDS
		.with_label_values(&[node_id.as_str()])
		.observe(lease_started_at.elapsed().as_secs_f64());

	result.map(|_| ())
}

async fn load_checkpoint_config(
	udb: Arc<universaldb::Database>,
	namespace_id: Option<Id>,
	actor_name: Option<String>,
	lease_ttl_ms: u64,
	semaphore: Arc<Semaphore>,
) -> Result<Option<CheckpointConfig>> {
	let Some(namespace_id) = namespace_id else {
		return Ok(None);
	};
	let namespace_config = udb
		.run(move |tx| async move {
			let tx = tx.with_subspace(namespace::keys::subspace());
			Ok(tx
				.read_opt(
					&namespace::keys::sqlite_config_key(namespace_id),
					universaldb::utils::IsolationLevel::Serializable,
				)
				.await?
				.unwrap_or_default())
		})
		.await?;

		Ok(Some(CheckpointConfig {
			namespace_config,
			semaphore,
			namespace_id: Some(namespace_id),
			actor_name,
			lease_ttl_ms,
		}))
}

#[cfg(debug_assertions)]
async fn maybe_validate_quota(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	compactor_config: &CompactorConfig,
	quota_validate_counts: &scc::HashMap<String, u32>,
	node_id: rivet_pools::NodeId,
) -> Result<()> {
	if compactor_config.quota_validate_every == 0 {
		return Ok(());
	}

	let pass_count = match quota_validate_counts.entry_async(actor_id.clone()).await {
		scc::hash_map::Entry::Occupied(mut entry) => {
			let next = entry.get().saturating_add(1);
			*entry.get_mut() = next;
			next
		}
		scc::hash_map::Entry::Vacant(entry) => {
			entry.insert_entry(1);
			1
		}
	};

	if pass_count % compactor_config.quota_validate_every == 0 {
		super::compact::validate_quota_with_node_id(udb, actor_id, node_id).await?;
	}

	Ok(())
}

async fn emit_metering_rollup(
	udb: Arc<universaldb::Database>,
	payload: SqliteCompactPayload,
	node_id: rivet_pools::NodeId,
) -> Result<()> {
	let actor_id = payload.actor_id;
	let node_id = node_id.to_string();
	let namespace_id = payload.namespace_id;
	let actor_name = payload.actor_name;
	let commit_bytes_since_rollup = payload.commit_bytes_since_rollup;
	let read_bytes_since_rollup = payload.read_bytes_since_rollup;

	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let node_id = node_id.clone();
		let actor_name = actor_name.clone();

		async move {
			let storage_used = quota::read_live(&tx, &actor_id).await?;
			metrics::SQLITE_STORAGE_USED_BYTES
				.with_label_values(&[node_id.as_str(), actor_id.as_str()])
				.set(storage_used as f64);
			let Some(namespace_id) = namespace_id else {
				tracing::debug!(
					actor_id = %actor_id,
					"skipping sqlite metering rollup without namespace id"
				);
				return Ok(());
			};
			let Some(actor_name) = actor_name else {
				tracing::debug!(
					actor_id = %actor_id,
					"skipping sqlite metering rollup without actor name"
				);
				return Ok(());
			};
			let namespace_tx = tx.with_subspace(namespace::keys::subspace());
			namespace::keys::metric::inc(
				&namespace_tx,
				namespace_id,
				namespace::keys::metric::Metric::SqliteStorageUsed(actor_name.clone()),
				storage_used,
			);
			namespace::keys::metric::inc(
				&namespace_tx,
				namespace_id,
				namespace::keys::metric::Metric::SqliteCommitBytes(actor_name.clone()),
				round_down_billable_bytes(commit_bytes_since_rollup)?,
			);
			namespace::keys::metric::inc(
				&namespace_tx,
				namespace_id,
				namespace::keys::metric::Metric::SqliteReadBytes(actor_name),
				round_down_billable_bytes(read_bytes_since_rollup)?,
			);

			Ok(())
		}
	})
	.await
}

fn round_down_billable_bytes(bytes: u64) -> Result<i64> {
	let rounded = bytes / util::metric::KV_BILLABLE_CHUNK * util::metric::KV_BILLABLE_CHUNK;
	i64::try_from(rounded).context("sqlite metering bytes exceeded i64")
}

fn spawn_renewal_task(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	holder_id: rivet_pools::NodeId,
	compactor_config: CompactorConfig,
	cancel_token: CancellationToken,
	deadline_tx: watch::Sender<tokio::time::Instant>,
) -> tokio::task::JoinHandle<()> {
	tokio::spawn(async move {
		let node_id = holder_id.to_string();
		let mut interval =
			tokio::time::interval(Duration::from_millis(compactor_config.lease_renew_interval_ms));
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
							tracing::warn!(?err, actor_id = %actor_id, "failed to compute sqlite compactor renewal timestamp");
							cancel_token.cancel();
							return;
						}
					};
					let renew_result = udb
						.run({
							let actor_id = actor_id.clone();
							move |tx| {
								let actor_id = actor_id.clone();
								async move {
									lease::renew(
										&tx,
										&actor_id,
										holder_id,
										compactor_config.lease_ttl_ms,
										now_ms,
									)
									.await
								}
							}
						})
						.await;

					match renew_result {
						Ok(lease::RenewOutcome::Renewed) => {
							metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL
								.with_label_values(&[node_id.as_str(), "ok"])
								.inc();
							match lease_deadline_after(&compactor_config) {
								Ok(deadline_after) => {
									let _ = deadline_tx.send(tokio::time::Instant::now() + deadline_after);
								}
								Err(err) => {
									tracing::warn!(?err, actor_id = %actor_id, "failed to compute sqlite compactor lease deadline");
									cancel_token.cancel();
									return;
								}
							}
						}
						Ok(outcome) => {
							metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL
								.with_label_values(&[node_id.as_str(), "stolen"])
								.inc();
							tracing::warn!(?outcome, actor_id = %actor_id, "sqlite compactor lease renewal stopped compaction");
							cancel_token.cancel();
							return;
						}
						Err(err) => {
							metrics::SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL
								.with_label_values(&[node_id.as_str(), "err"])
								.inc();
							tracing::warn!(?err, actor_id = %actor_id, "sqlite compactor lease renewal failed");
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
					tracing::warn!("sqlite compactor lease local deadline elapsed");
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

fn lease_deadline_after(compactor_config: &CompactorConfig) -> Result<Duration> {
	let ttl = Duration::from_millis(compactor_config.lease_ttl_ms);
	let margin = Duration::from_millis(compactor_config.lease_margin_ms);
	ttl
		.checked_sub(margin)
		.context("sqlite compactor lease margin must be less than ttl")
}

fn now_ms() -> Result<i64> {
	let elapsed = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite compactor timestamp exceeded i64")
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use std::{future::Future, pin::Pin};

	use parking_lot::Mutex;

	use super::*;

	type HookFuture = Pin<Box<dyn Future<Output = Result<bool>> + Send>>;
	type AdminOpHook = dyn Fn(AdminOpHookEvent) -> HookFuture + Send + Sync + 'static;

	static ADMIN_OP_HOOK: Mutex<Option<Arc<AdminOpHook>>> = Mutex::new(None);
	static RUN_READY_SIGNAL: Mutex<Option<tokio::sync::oneshot::Sender<()>>> = Mutex::new(None);

	#[derive(Clone, Debug)]
	pub struct AdminOpHookEvent {
		pub request: SqliteOpRequest,
		pub record: admin::AdminOpRecord,
		pub resume: Option<AdminResumeState>,
	}

	pub struct AdminOpHookGuard;

	impl Drop for AdminOpHookGuard {
		fn drop(&mut self) {
			*ADMIN_OP_HOOK.lock() = None;
		}
	}

	pub fn set_admin_op_hook<F, Fut>(hook: F) -> AdminOpHookGuard
	where
		F: Fn(AdminOpHookEvent) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<bool>> + Send + 'static,
	{
		*ADMIN_OP_HOOK.lock() = Some(Arc::new(move |event| Box::pin(hook(event))));
		AdminOpHookGuard
	}

	pub fn set_run_ready_signal(tx: tokio::sync::oneshot::Sender<()>) {
		*RUN_READY_SIGNAL.lock() = Some(tx);
	}

	pub(super) fn notify_run_ready() {
		if let Some(tx) = RUN_READY_SIGNAL.lock().take() {
			let _ = tx.send(());
		}
	}

	pub(super) async fn maybe_handle_admin_op(event: AdminOpHookEvent) -> Result<bool> {
		let hook = ADMIN_OP_HOOK.lock().clone();
		match hook {
			Some(hook) => hook(event).await,
			None => Ok(false),
		}
	}

	pub async fn handle_admin_op_once(
		udb: Arc<universaldb::Database>,
		request: SqliteOpRequest,
		holder_id: rivet_pools::NodeId,
	) -> Result<()> {
		super::handle_admin_op(udb, request, holder_id).await
	}

	pub async fn handle_admin_ops_with_limit_for_test(
		udb: Arc<universaldb::Database>,
		requests: Vec<SqliteOpRequest>,
		holder_id: rivet_pools::NodeId,
		max_concurrent_workers: usize,
	) -> Result<()> {
		let semaphore = Arc::new(Semaphore::new(max_concurrent_workers.max(1)));
		let mut workers = JoinSet::new();

		for request in requests {
			let udb = Arc::clone(&udb);
			let semaphore = Arc::clone(&semaphore);
			workers.spawn(async move {
				let _permit = semaphore
					.acquire_owned()
					.await
					.context("sqlite admin op semaphore closed")?;
				super::handle_admin_op(udb, request, holder_id).await
			});
		}

		while let Some(join_result) = workers.join_next().await {
			join_result.context("sqlite admin op worker panicked")??;
		}

		Ok(())
	}

	pub async fn run_for_test(
		udb: Arc<universaldb::Database>,
		ups: Ups,
		compactor_config: CompactorConfig,
		holder_id: rivet_pools::NodeId,
	) -> Result<()> {
		super::run_with_node_id(
			udb,
			ups,
			TermSignal::get(),
			compactor_config,
			holder_id,
		)
		.await
	}

	pub async fn unsubscribe_probe_for_test(ups: Ups) -> Result<()> {
		let mut compact_sub = ups
			.queue_subscribe(SqliteCompactSubject, COMPACTOR_QUEUE_GROUP)
			.await?;
		match compact_sub.next().await? {
			NextOutput::Message(_) => Ok(()),
			NextOutput::Unsubscribed => {
				Err(anyhow::anyhow!("sqlite compactor compact sub unsubscribed"))
			}
		}
	}

	pub async fn handle_trigger_once(
		udb: Arc<universaldb::Database>,
		actor_id: String,
		compactor_config: CompactorConfig,
		cancel_token: CancellationToken,
	) -> Result<()> {
		handle_payload_once(
			udb,
			SqliteCompactPayload {
				actor_id,
				namespace_id: None,
				actor_name: None,
				commit_bytes_since_rollup: 0,
				read_bytes_since_rollup: 0,
			},
			compactor_config,
			cancel_token,
		)
		.await
	}

	pub async fn handle_payload_once(
		udb: Arc<universaldb::Database>,
		payload: SqliteCompactPayload,
		compactor_config: CompactorConfig,
		cancel_token: CancellationToken,
	) -> Result<()> {
		handle_payload_once_with_checkpoint_semaphore(
			udb,
			payload,
			compactor_config,
			cancel_token,
			Arc::new(Semaphore::new(16)),
		)
		.await
	}

	pub async fn handle_payload_once_with_checkpoint_semaphore(
		udb: Arc<universaldb::Database>,
		payload: SqliteCompactPayload,
		compactor_config: CompactorConfig,
		cancel_token: CancellationToken,
		checkpoint_semaphore: Arc<Semaphore>,
	) -> Result<()> {
		handle_trigger(
			udb,
			payload,
			compactor_config,
			rivet_pools::NodeId::new(),
			cancel_token,
			checkpoint_semaphore,
			#[cfg(debug_assertions)]
			Arc::new(scc::HashMap::new()),
		)
		.await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_config_matches_spec() {
		let config = CompactorConfig::default();

		assert_eq!(config.lease_ttl_ms, 30_000);
		assert_eq!(config.lease_renew_interval_ms, 10_000);
		assert_eq!(config.lease_margin_ms, 5_000);
		assert_eq!(config.compaction_delta_threshold, 32);
		assert_eq!(config.batch_size_deltas, 32);
		assert_eq!(config.max_concurrent_workers, 64);
		assert!(!config.pitr_enabled);
		assert_eq!(config.max_concurrent_checkpoints, 16);
		assert_eq!(config.ups_subject, "sqlite.compact");
		#[cfg(debug_assertions)]
		assert_eq!(config.quota_validate_every, 16);
	}
}
