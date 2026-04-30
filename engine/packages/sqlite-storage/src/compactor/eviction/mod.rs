use std::{
	collections::BTreeMap,
	ops::Deref,
	sync::Arc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use gas::prelude::{Database, Id, StandaloneCtx, db};
use rivet_pools::NodeId;
use rivet_runtime::TermSignal;
use tokio_util::sync::CancellationToken;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use crate::pump::{
	constants::{ACCESS_TOUCH_THROTTLE_MS, HOT_CACHE_WINDOW_MS, SHARD_RETENTION_MARGIN},
	keys::{self, CompactorQueueKind},
	types::DatabaseBranchId,
	udb,
};

use super::{CompactorLease, TakeOutcome, decode_lease, encode_lease, metrics};

const EVICTION_COMPACTOR_NAME: &str = "sqlite_eviction_compactor";
const VERSIONSTAMP_ZERO: [u8; 16] = [0; 16];
const VERSIONSTAMP_INFINITY: [u8; 16] = [0xff; 16];

#[derive(Clone, Debug)]
pub struct EvictionCompactorConfig {
	pub lease_ttl_ms: u64,
	pub sweep_interval_ms: u64,
	pub batch_size: usize,
}

impl Default for EvictionCompactorConfig {
	fn default() -> Self {
		Self {
			lease_ttl_ms: 30_000,
			sweep_interval_ms: 30_000,
			batch_size: 256,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictionCandidate {
	pub last_access_bucket: i64,
	pub branch_id: DatabaseBranchId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictableShardVersion {
	pub branch_id: DatabaseBranchId,
	pub last_access_bucket: i64,
	pub shard_id: u32,
	pub as_of_txid: u64,
	pub last_hot_pass_txid_at_plan: u64,
	pub shard_value: Vec<u8>,
	pub pidx_deletes: Vec<EvictablePidxEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictablePidxEntry {
	pub key: Vec<u8>,
	pub expected_value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictionSweepOutcome {
	pub lease_acquired: bool,
	pub scanned_candidates: Vec<EvictionCandidate>,
	pub evictable_shard_versions: Vec<EvictableShardVersion>,
}

#[tracing::instrument(skip_all)]
pub async fn start(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	eviction_config: EvictionCompactorConfig,
) -> Result<()> {
	let node_id = pools.node_id();
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		EVICTION_COMPACTOR_NAME,
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	run_with_node_id(
		Arc::new(ctx.udb()?.deref().clone()),
		TermSignal::get(),
		eviction_config,
		node_id,
	)
	.await
}

#[tracing::instrument(skip_all)]
#[allow(dead_code)]
pub(crate) async fn run(
	udb: Arc<universaldb::Database>,
	term_signal: TermSignal,
	eviction_config: EvictionCompactorConfig,
) -> Result<()> {
	run_with_node_id(udb, term_signal, eviction_config, NodeId::new()).await
}

async fn run_with_node_id(
	udb: Arc<universaldb::Database>,
	mut term_signal: TermSignal,
	eviction_config: EvictionCompactorConfig,
	holder_id: NodeId,
) -> Result<()> {
	let mut interval = tokio::time::interval(Duration::from_millis(
		eviction_config.sweep_interval_ms,
	));
	interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	let shutdown = CancellationToken::new();

	loop {
		tokio::select! {
			_ = term_signal.recv() => return Ok(()),
			_ = interval.tick() => {
				if let Err(err) = sweep_once(
					udb.as_ref(),
					&eviction_config,
					holder_id,
					shutdown.child_token(),
				).await {
					tracing::warn!(?err, "sqlite eviction compactor sweep failed");
				}
			}
		}
	}
}

pub async fn sweep_once(
	udb: &universaldb::Database,
	eviction_config: &EvictionCompactorConfig,
	holder_id: NodeId,
	cancel_token: CancellationToken,
) -> Result<EvictionSweepOutcome> {
	if cancel_token.is_cancelled() {
		anyhow::bail!("sqlite eviction compaction cancelled");
	}

	let now_ms = now_ms()?;
	let lease_ttl_ms = eviction_config.lease_ttl_ms;
	let take_outcome = udb
		.run(move |tx| async move {
			take_global_lease(&tx, holder_id, lease_ttl_ms, now_ms).await
		})
		.await?;

	if matches!(take_outcome, TakeOutcome::Skip) {
		return Ok(EvictionSweepOutcome {
			lease_acquired: false,
			scanned_candidates: Vec::new(),
			evictable_shard_versions: Vec::new(),
		});
	}

	let result = async {
		let holder_label = holder_id.to_string();
		let _timer = metrics::SQLITE_EVICTION_PASS_DURATION
			.with_label_values(&[holder_label.as_str()])
			.start_timer();
		let (scanned_candidates, evictable_shard_versions) =
			scan_eviction_index(udb, eviction_config.batch_size, now_ms, cancel_token).await?;
		let evictable_shard_versions =
			clear_evictable_shard_versions(udb, evictable_shard_versions, holder_id).await?;
		metrics::SQLITE_EVICTION_PASS_SHARDS_CLEARED_TOTAL
			.with_label_values(&[holder_label.as_str()])
			.inc_by(evictable_shard_versions.len() as u64);
		Ok::<_, anyhow::Error>((scanned_candidates, evictable_shard_versions))
	}
	.await;
	let release_result = udb
		.run(move |tx| async move { release_global_lease(&tx).await })
		.await;

	if let Err(err) = release_result {
		tracing::warn!(?err, "failed to release sqlite eviction compactor lease");
	}

	let (scanned_candidates, evictable_shard_versions) = result?;
	tracing::debug!(
		node_id = %holder_id,
		scanned_candidates = scanned_candidates.len(),
		evictable_shard_versions = evictable_shard_versions.len(),
		batch_size = eviction_config.batch_size,
		"sqlite eviction compactor scanned eviction index"
	);

	Ok(EvictionSweepOutcome {
		lease_acquired: true,
		scanned_candidates,
		evictable_shard_versions,
	})
}

async fn take_global_lease(
	tx: &universaldb::Transaction,
	holder_id: NodeId,
	ttl_ms: u64,
	now_ms: i64,
) -> Result<TakeOutcome> {
	let key = keys::compactor_global_lease_key(CompactorQueueKind::Eviction);
	let current = tx.informal().get(&key, Serializable).await?;

	if let Some(current) = current {
		let lease = decode_lease(&current)?;
		if lease.holder_id != holder_id && lease.expires_at_ms > now_ms {
			return Ok(TakeOutcome::Skip);
		}
	}

	let lease = CompactorLease {
		holder_id,
		expires_at_ms: expires_at_ms(now_ms, ttl_ms)?,
	};
	tx.informal().set(&key, &encode_lease(lease)?);

	Ok(TakeOutcome::Acquired)
}

async fn release_global_lease(tx: &universaldb::Transaction) -> Result<()> {
	tx.informal()
		.clear(&keys::compactor_global_lease_key(CompactorQueueKind::Eviction));
	Ok(())
}

async fn scan_eviction_index(
	udb: &universaldb::Database,
	batch_size: usize,
	now_ms: i64,
	cancel_token: CancellationToken,
) -> Result<(Vec<EvictionCandidate>, Vec<EvictableShardVersion>)> {
	if batch_size == 0 {
		return Ok((Vec::new(), Vec::new()));
	}

	udb.run(move |tx| {
		let cancel_token = cancel_token.clone();
		async move {
			if cancel_token.is_cancelled() {
				anyhow::bail!("sqlite eviction compaction cancelled");
			}

			let prefix = keys::ctr_eviction_index_prefix();
			let prefix_subspace =
				universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
			let informal = tx.informal();
			let mut stream = informal.get_ranges_keyvalues(
				RangeOption {
					limit: Some(batch_size),
					mode: StreamingMode::WantAll,
					..RangeOption::from(&prefix_subspace)
				},
				Snapshot,
			);
			let mut rows = Vec::new();
			let mut evictable = Vec::new();

			while let Some(entry) = stream.try_next().await? {
				let (last_access_bucket, branch_id) =
					keys::decode_ctr_eviction_index_key(entry.key())?;
				evictable.extend(
					plan_evictable_shard_versions(&tx, branch_id, last_access_bucket, now_ms)
						.await?,
				);
				rows.push(EvictionCandidate {
					last_access_bucket,
					branch_id,
				});
			}

			Ok((rows, evictable))
		}
	})
	.await
}

async fn plan_evictable_shard_versions(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	last_access_bucket: i64,
	now_ms: i64,
) -> Result<Vec<EvictableShardVersion>> {
	if !is_past_hot_cache_window(last_access_bucket, now_ms)? {
		return Ok(Vec::new());
	}

	let cold_drained_txid = read_u64_be(
		tx,
		&keys::branch_manifest_cold_drained_txid_key(branch_id),
		Serializable,
	)
	.await?
	.unwrap_or(0);
	let last_hot_pass_txid = read_u64_be(
		tx,
		&keys::branch_manifest_last_hot_pass_txid_key(branch_id),
		Serializable,
	)
	.await?
	.unwrap_or(0);
	let desc_pin_txid = read_pin_txid(tx, branch_id, &keys::branches_desc_pin_key(branch_id)).await?;
	let bk_pin_txid = read_pin_txid(tx, branch_id, &keys::branches_bk_pin_key(branch_id)).await?;
	let shard_versions = load_branch_shard_versions(tx, branch_id).await?;
	let pidx_rows = load_branch_pidx_rows(tx, branch_id).await?;

	let mut evictable = Vec::new();
	for (idx, version) in shard_versions.iter().enumerate() {
		let newer_version_exists = shard_versions[idx + 1..]
			.iter()
			.any(|newer| newer.shard_id == version.shard_id);
		if !newer_version_exists {
			continue;
		}
		if cold_drained_txid < version.as_of_txid {
			continue;
		}
		if last_hot_pass_txid.saturating_sub(SHARD_RETENTION_MARGIN) < version.as_of_txid {
			continue;
		}
		if is_pinned(version.as_of_txid, desc_pin_txid) || is_pinned(version.as_of_txid, bk_pin_txid) {
			continue;
		}

		evictable.push(EvictableShardVersion {
			branch_id,
			last_access_bucket,
			shard_id: version.shard_id,
			as_of_txid: version.as_of_txid,
			last_hot_pass_txid_at_plan: last_hot_pass_txid,
			shard_value: version.value.clone(),
			pidx_deletes: pidx_rows
				.iter()
				.filter(|row| {
					row.shard_id == version.shard_id && row.owner_txid <= version.as_of_txid
				})
				.map(|row| EvictablePidxEntry {
					key: row.key.clone(),
					expected_value: row.value.clone(),
				})
				.collect(),
		});
	}

	Ok(evictable)
}

async fn clear_evictable_shard_versions(
	udb: &universaldb::Database,
	evictable_shard_versions: Vec<EvictableShardVersion>,
	holder_id: NodeId,
) -> Result<Vec<EvictableShardVersion>> {
	if evictable_shard_versions.is_empty() {
		return Ok(evictable_shard_versions);
	}

	let clear_outcome = udb
		.run({
			let evictable_shard_versions = evictable_shard_versions.clone();
			move |tx| {
				let evictable_shard_versions = evictable_shard_versions.clone();
				async move {
					let planned_hot_pass_txids =
						planned_hot_pass_txids_by_branch(&evictable_shard_versions);
					for (branch_id, planned_txid) in planned_hot_pass_txids {
						let current_txid = read_u64_be(
							&tx,
							&keys::branch_manifest_last_hot_pass_txid_key(branch_id),
							Serializable,
						)
						.await?
						.unwrap_or(0);
						if current_txid != planned_txid {
							return Ok(EvictionClearOutcome::HotPassAdvanced);
						}
					}
					let evictable_shard_versions =
						filter_now_pinned_versions(&tx, &evictable_shard_versions).await?;
					let fully_evicted_index_keys =
						fully_evicted_index_keys_after_clear(&tx, &evictable_shard_versions)
							.await?;

					for version in &evictable_shard_versions {
						for pidx in &version.pidx_deletes {
							udb::compare_and_clear(&tx, &pidx.key, &pidx.expected_value);
						}
						udb::compare_and_clear(
							&tx,
							&keys::branch_shard_key(
								version.branch_id,
								version.shard_id,
								version.as_of_txid,
							),
							&version.shard_value,
						);
					}
					for key in fully_evicted_index_keys {
						tx.informal().clear(&key);
					}

					Ok(EvictionClearOutcome::Cleared(evictable_shard_versions))
				}
			}
		})
		.await?;

	match clear_outcome {
		EvictionClearOutcome::Cleared(cleared_versions) => Ok(cleared_versions),
		EvictionClearOutcome::HotPassAdvanced => {
			let holder_label = holder_id.to_string();
			metrics::SQLITE_EVICTION_OCC_ABORT_TOTAL
				.with_label_values(&[holder_label.as_str(), "hot_pass_advanced"])
				.inc();
			Ok(Vec::new())
		}
	}
}

async fn filter_now_pinned_versions(
	tx: &universaldb::Transaction,
	evictable_shard_versions: &[EvictableShardVersion],
) -> Result<Vec<EvictableShardVersion>> {
	let mut pins_by_branch = BTreeMap::new();
	for version in evictable_shard_versions {
		if pins_by_branch.contains_key(&version.branch_id) {
			continue;
		}
		let desc_pin_txid =
			read_pin_txid(tx, version.branch_id, &keys::branches_desc_pin_key(version.branch_id))
				.await?;
		let bk_pin_txid =
			read_pin_txid(tx, version.branch_id, &keys::branches_bk_pin_key(version.branch_id))
				.await?;
		pins_by_branch.insert(version.branch_id, (desc_pin_txid, bk_pin_txid));
	}

	Ok(evictable_shard_versions
		.iter()
		.filter(|version| {
			let Some((desc_pin_txid, bk_pin_txid)) = pins_by_branch.get(&version.branch_id) else {
				return false;
			};
			!is_pinned(version.as_of_txid, *desc_pin_txid)
				&& !is_pinned(version.as_of_txid, *bk_pin_txid)
		})
		.cloned()
		.collect())
}

fn planned_hot_pass_txids_by_branch(
	evictable_shard_versions: &[EvictableShardVersion],
) -> BTreeMap<DatabaseBranchId, u64> {
	let mut planned = BTreeMap::new();
	for version in evictable_shard_versions {
		planned
			.entry(version.branch_id)
			.or_insert(version.last_hot_pass_txid_at_plan);
	}
	planned
}

async fn fully_evicted_index_keys_after_clear(
	tx: &universaldb::Transaction,
	evictable_shard_versions: &[EvictableShardVersion],
) -> Result<Vec<Vec<u8>>> {
	let mut planned = BTreeMap::<DatabaseBranchId, PlannedBranchEviction>::new();
	for version in evictable_shard_versions {
		let branch_plan = planned
			.entry(version.branch_id)
			.or_insert_with(PlannedBranchEviction::default);
		branch_plan.last_access_bucket = Some(version.last_access_bucket);
		branch_plan.shard_versions.insert(
			(version.shard_id, version.as_of_txid),
			version.shard_value.clone(),
		);
	}

	let mut keys = Vec::new();
	for (branch_id, branch_plan) in planned {
		let Some(last_access_bucket) = branch_plan.last_access_bucket else {
			continue;
		};
		let current_shards = load_branch_shard_versions(tx, branch_id).await?;
		if current_shards.is_empty() {
			continue;
		}
		let all_current_shards_planned = current_shards.iter().all(|shard| {
			branch_plan
				.shard_versions
				.get(&(shard.shard_id, shard.as_of_txid))
				.is_some_and(|expected_value| expected_value == &shard.value)
		});
		if all_current_shards_planned {
			keys.push(keys::ctr_eviction_index_key(last_access_bucket, branch_id));
		}
	}

	Ok(keys)
}

#[derive(Debug, Default)]
struct PlannedBranchEviction {
	last_access_bucket: Option<i64>,
	shard_versions: BTreeMap<(u32, u64), Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvictionClearOutcome {
	Cleared(Vec<EvictableShardVersion>),
	HotPassAdvanced,
}

fn is_past_hot_cache_window(last_access_bucket: i64, now_ms: i64) -> Result<bool> {
	let last_access_ms = last_access_bucket
		.checked_mul(ACCESS_TOUCH_THROTTLE_MS)
		.context("sqlite eviction access bucket timestamp overflowed")?;
	Ok(now_ms.saturating_sub(last_access_ms) >= HOT_CACHE_WINDOW_MS)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BranchShardVersion {
	shard_id: u32,
	as_of_txid: u64,
	value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BranchPidxRow {
	shard_id: u32,
	owner_txid: u64,
	key: Vec<u8>,
	value: Vec<u8>,
}

async fn load_branch_shard_versions(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<Vec<BranchShardVersion>> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut versions = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		if let Some((shard_id, as_of_txid)) =
			decode_branch_shard_version_key(branch_id, entry.key())?
		{
			versions.push(BranchShardVersion {
				shard_id,
				as_of_txid,
				value: entry.value().to_vec(),
			});
		}
	}

	Ok(versions)
}

async fn load_branch_pidx_rows(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<Vec<BranchPidxRow>> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		let pgno = decode_branch_pidx_pgno(branch_id, entry.key())?;
		let value = entry.value().to_vec();
		let owner_txid = decode_pidx_txid(&value)?;
		rows.push(BranchPidxRow {
			shard_id: pgno / keys::SHARD_SIZE,
			owner_txid,
			key: entry.key().to_vec(),
			value,
		});
	}

	Ok(rows)
}

fn decode_branch_shard_version_key(
	branch_id: DatabaseBranchId,
	key: &[u8],
) -> Result<Option<(u32, u64)>> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch shard key did not start with expected prefix")?;
	if suffix.len() == std::mem::size_of::<u32>() {
		return Ok(None);
	}
	if suffix.len() != std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
		|| suffix[std::mem::size_of::<u32>()] != b'/'
	{
		anyhow::bail!("branch shard version key suffix had invalid length");
	}

	let shard_id = u32::from_be_bytes(
		suffix[..std::mem::size_of::<u32>()]
			.try_into()
			.context("decode branch shard id")?,
	);
	let as_of_txid = u64::from_be_bytes(
		suffix[std::mem::size_of::<u32>() + 1..]
			.try_into()
			.context("decode branch shard txid")?,
	);

	Ok(Some((shard_id, as_of_txid)))
}

fn decode_branch_pidx_pgno(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch pidx key did not start with expected prefix")?;
	let suffix: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.context("branch pidx key suffix had invalid length")?;

	Ok(u32::from_be_bytes(suffix))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; std::mem::size_of::<u64>()] = value
		.try_into()
		.context("pidx txid had invalid length")?;

	Ok(u64::from_be_bytes(bytes))
}

async fn read_u64_be(
	tx: &universaldb::Transaction,
	key: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<u64>> {
	let Some(bytes) = tx.informal().get(key, isolation_level).await? else {
		return Ok(None);
	};
	let bytes: [u8; std::mem::size_of::<u64>()] = Vec::<u8>::from(bytes)
		.as_slice()
		.try_into()
		.context("sqlite eviction txid value should be exactly 8 bytes")?;

	Ok(Some(u64::from_be_bytes(bytes)))
}

async fn read_pin_txid(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	pin_key: &[u8],
) -> Result<Option<u64>> {
	let Some(bytes) = tx.informal().get(pin_key, Serializable).await? else {
		return Ok(None);
	};
	let pin: [u8; 16] = Vec::<u8>::from(bytes)
		.as_slice()
		.try_into()
		.context("sqlite branch pin should be exactly 16 bytes")?;
	if pin == VERSIONSTAMP_ZERO || pin == VERSIONSTAMP_INFINITY {
		return Ok(None);
	}

	let Some(txid) = read_u64_be(tx, &keys::branch_vtx_key(branch_id, pin), Serializable).await?
	else {
		return Ok(Some(0));
	};

	Ok(Some(txid))
}

fn is_pinned(as_of_txid: u64, pin_txid: Option<u64>) -> bool {
	pin_txid.is_some_and(|pin_txid| pin_txid <= as_of_txid)
}

fn expires_at_ms(now_ms: i64, ttl_ms: u64) -> Result<i64> {
	let ttl_ms =
		i64::try_from(ttl_ms).context("sqlite eviction compactor lease ttl overflowed i64")?;
	now_ms
		.checked_add(ttl_ms)
		.context("sqlite eviction compactor lease expiration overflowed i64")
}

fn now_ms() -> Result<i64> {
	let elapsed = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite eviction compactor timestamp exceeded i64")
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use super::*;

	pub async fn sweep_once_for_test(
		udb: &universaldb::Database,
		eviction_config: &EvictionCompactorConfig,
		holder_id: NodeId,
		cancel_token: CancellationToken,
	) -> Result<EvictionSweepOutcome> {
		sweep_once(udb, eviction_config, holder_id, cancel_token).await
	}

	pub async fn plan_evictable_shard_versions_for_test(
		udb: &universaldb::Database,
		branch_id: DatabaseBranchId,
		last_access_bucket: i64,
		now_ms: i64,
	) -> Result<Vec<EvictableShardVersion>> {
		udb.run(move |tx| async move {
			plan_evictable_shard_versions(&tx, branch_id, last_access_bucket, now_ms).await
		})
		.await
	}

	pub async fn clear_evictable_shard_versions_for_test(
		udb: &universaldb::Database,
		evictable_shard_versions: Vec<EvictableShardVersion>,
	) -> Result<Vec<EvictableShardVersion>> {
		clear_evictable_shard_versions(
			udb,
			evictable_shard_versions,
			NodeId::from(uuid::Uuid::nil()),
		)
		.await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_config_matches_scaffold() {
		let config = EvictionCompactorConfig::default();

		assert_eq!(config.lease_ttl_ms, 30_000);
		assert_eq!(config.sweep_interval_ms, 30_000);
		assert_eq!(config.batch_size, 256);
	}
}
