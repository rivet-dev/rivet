mod fault_common;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use sqlite_storage::{
	cold_tier::{FaultyColdTier, FilesystemColdTier},
	compactor::cold::{ColdCompactorConfig, worker},
	keys::branch_meta_cold_compact_key,
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn cold_compactor_phase_a_pending_put_5s() -> Result<()> {
	tokio::time::pause();

	let db = Arc::new(fault_common::test_db("sqlite-storage-cold-a-latency-").await?);
	fault_common::seed_cold_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-storage-cold-a-latency-").tempdir()?;
	let tier = Arc::new(FaultyColdTier::new(FilesystemColdTier::new(cold_root.path())));
	tier.set_latency(Duration::from_secs(5));
	let config = ColdCompactorConfig {
		lease_ttl_ms: 10_000,
		lease_renew_interval_ms: 6_000,
		lease_margin_ms: 1_000,
		..fault_common::cold_config()
	};

	let handle = tokio::spawn(worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		fault_common::cold_payload(),
		config,
		CancellationToken::new(),
		tier,
	));

	let mut handoff_committed = false;
	for _ in 0..16 {
		tokio::task::yield_now().await;
		if fault_common::read_value(
			&db,
			branch_meta_cold_compact_key(fault_common::database_branch_id()),
		)
		.await?
		.is_some()
		{
			handoff_committed = true;
			break;
		}
	}
	assert!(
		handoff_committed,
		"Phase A should commit in-flight uuid before the 5s pending-marker PUT latency"
	);

	tokio::time::advance(Duration::from_secs(5)).await;
	handle.await??;

	Ok(())
}
