mod fault_common;

use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use async_trait::async_trait;
use sqlite_storage::{
	cold_tier::{ColdTier, ColdTierObjectMetadata, FilesystemColdTier},
	compactor::cold::{decode_cold_compact_state, worker},
	keys::{branch_manifest_cold_drained_txid_key, branch_meta_cold_compact_key},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct CancelBeforePhaseCTier {
	inner: FilesystemColdTier,
	cancel: CancellationToken,
	cancelled: Arc<AtomicBool>,
}

#[async_trait]
impl ColdTier for CancelBeforePhaseCTier {
	async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
		self.inner.put_object(key, bytes).await?;
		if key.contains("/pointer_snapshot/") && !self.cancelled.swap(true, Ordering::SeqCst) {
			self.cancel.cancel();
		}
		Ok(())
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
async fn cold_compactor_lease_loss_b_to_c() -> Result<()> {
	let db = Arc::new(fault_common::test_db("sqlite-storage-cold-lease-loss-").await?);
	fault_common::seed_cold_branch(&db).await?;
	let cold_root = Builder::new().prefix("sqlite-storage-cold-lease-loss-").tempdir()?;
	let cancel = CancellationToken::new();
	let tier = Arc::new(CancelBeforePhaseCTier {
		inner: FilesystemColdTier::new(cold_root.path()),
		cancel: cancel.clone(),
		cancelled: Arc::new(AtomicBool::new(false)),
	});

	let err = worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		fault_common::cold_payload(),
		fault_common::cold_config(),
		cancel,
		tier,
	)
	.await
	.expect_err("lease-loss cancellation should stop before Phase C");
	assert!(
		format!("{err:?}").contains("cancelled"),
		"unexpected error: {err:?}"
	);
	assert!(
		fault_common::read_value(
			&db,
			sqlite_storage::keys::branch_meta_cold_lease_key(fault_common::database_branch_id())
		)
		.await?
		.is_none(),
		"cancelled pod should release the cold lease"
	);
	assert_eq!(
		fault_common::read_u64_be(
			&db,
			branch_manifest_cold_drained_txid_key(fault_common::database_branch_id())
		)
		.await?,
		Some(3)
	);

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&db),
		fault_common::cold_payload(),
		fault_common::cold_config(),
		CancellationToken::new(),
		Arc::new(FilesystemColdTier::new(cold_root.path())),
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
