mod fault_common;

use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use async_trait::async_trait;
use sqlite_storage::{
	cold_tier::{ColdTier, ColdTierObjectMetadata, ColdTierOperation, FaultyColdTier, FilesystemColdTier},
	compactor::{
		cold::{decode_cold_compact_state, worker},
		metrics,
	},
	keys::{branch_manifest_cold_drained_txid_key, branch_meta_cold_compact_key},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct FailPhaseBPutTier {
	inner: FaultyColdTier<FilesystemColdTier>,
	armed: Arc<AtomicBool>,
}

#[async_trait]
impl ColdTier for FailPhaseBPutTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		if key.ends_with(".marker") && !self.armed.swap(true, Ordering::SeqCst) {
			self.inner.put_object(key, bytes).await?;
			self.inner.fail_operation(ColdTierOperation::Put, true);
			return Ok(());
		}

		self.inner.put_object(key, bytes).await
	}

	async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
		self.inner.get_object(key).await
	}

	async fn delete_objects(&self, keys: &[String]) -> Result<()> {
		self.inner.delete_objects(keys).await
	}

	async fn list_prefix(&self, prefix: &str) -> Result<Vec<ColdTierObjectMetadata>> {
		self.inner.list_prefix(prefix).await
	}
}

#[tokio::test]
async fn cold_compactor_5xx_phase_b() -> Result<()> {
	let db = Arc::new(fault_common::test_db("sqlite-storage-cold-5xx-").await?);
	fault_common::seed_cold_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-storage-cold-5xx-").tempdir()?;
	let faulty = FaultyColdTier::new(FilesystemColdTier::new(cold_root.path()));
	let tier = Arc::new(FailPhaseBPutTier {
		inner: faulty.clone(),
		armed: Arc::new(AtomicBool::new(false)),
	});
	let failures_before = metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
		.with_label_values(&["unknown", "put"])
		.get();

	let err = worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		fault_common::cold_payload(),
		fault_common::cold_config(),
		CancellationToken::new(),
		tier,
	)
	.await
	.expect_err("phase B PUT failure should abort the cold pass");
	assert!(
		format!("{err:?}").contains("injected cold-tier failure"),
		"unexpected error: {err:?}"
	);
	assert_eq!(
		metrics::SQLITE_S3_REQUEST_FAILURES_TOTAL
			.with_label_values(&["unknown", "put"])
			.get(),
		failures_before + 1
	);
	assert!(
		fault_common::read_value(
			&db,
			sqlite_storage::keys::branch_meta_cold_lease_key(fault_common::database_branch_id())
		)
		.await?
		.is_none(),
		"cold lease should be released after a failed pass"
	);
	assert_eq!(
		fault_common::read_u64_be(
			&db,
			branch_manifest_cold_drained_txid_key(fault_common::database_branch_id())
		)
		.await?,
		Some(3)
	);

	faulty.fail_operation(ColdTierOperation::Put, false);
	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		fault_common::cold_payload(),
		fault_common::cold_config(),
		CancellationToken::new(),
		Arc::new(faulty),
	)
	.await?;
	assert_eq!(
		fault_common::read_u64_be(
			&db,
			branch_manifest_cold_drained_txid_key(fault_common::database_branch_id())
		)
		.await?,
		Some(7)
	);
	let state = fault_common::read_value(
		&db,
		branch_meta_cold_compact_key(fault_common::database_branch_id()),
	)
	.await?
	.expect("cold compact state should exist");
	assert_eq!(decode_cold_compact_state(&state)?.in_flight_uuid, None);

	Ok(())
}
