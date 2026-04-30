use std::{
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
	keys::{self, CompactorQueueKind},
	types::ActorBranchId,
};

use super::{CompactorLease, TakeOutcome, decode_lease, encode_lease};

const EVICTION_COMPACTOR_NAME: &str = "sqlite_eviction_compactor";

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
	pub branch_id: ActorBranchId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictionSweepOutcome {
	pub lease_acquired: bool,
	pub scanned_candidates: Vec<EvictionCandidate>,
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
		});
	}

	let result = scan_eviction_index(udb, eviction_config.batch_size, cancel_token).await;
	let release_result = udb
		.run(move |tx| async move { release_global_lease(&tx).await })
		.await;

	if let Err(err) = release_result {
		tracing::warn!(?err, "failed to release sqlite eviction compactor lease");
	}

	let scanned_candidates = result?;
	tracing::debug!(
		node_id = %holder_id,
		scanned_candidates = scanned_candidates.len(),
		batch_size = eviction_config.batch_size,
		"sqlite eviction compactor scanned eviction index"
	);

	Ok(EvictionSweepOutcome {
		lease_acquired: true,
		scanned_candidates,
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
	cancel_token: CancellationToken,
) -> Result<Vec<EvictionCandidate>> {
	if batch_size == 0 {
		return Ok(Vec::new());
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

			while let Some(entry) = stream.try_next().await? {
				let (last_access_bucket, branch_id) =
					keys::decode_ctr_eviction_index_key(entry.key())?;
				rows.push(EvictionCandidate {
					last_access_bucket,
					branch_id,
				});
			}

			Ok(rows)
		}
	})
	.await
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
