mod fault_common;

use std::sync::Arc;

use anyhow::{Context, Result};
use gas::prelude::Id;
use depot::{
	cold_tier::{ColdTier, FilesystemColdTier},
	compactor::cold::worker,
	keys::{
		database_pointer_cur_key, branch_meta_head_key, branch_shard_key, branches_list_key,
		namespace_pointer_cur_key,
	},
	types::{
		DatabasePointer, DBHead, NamespaceId, NamespacePointer, decode_database_branch_record,
		decode_pointer_snapshot, encode_database_pointer, encode_db_head, encode_namespace_pointer,
	},
};
use tempfile::Builder;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn dr_replay_from_s3_alone() -> Result<()> {
	let source = Arc::new(fault_common::test_db("depot-dr-source-").await?);
	fault_common::seed_cold_branch(&source).await?;
	let cold_root = Builder::new().prefix("depot-dr-cold-").tempdir()?;
	let tier = Arc::new(FilesystemColdTier::new(cold_root.path()));

	worker::test_hooks::handle_payload_once_with_cold_tier(
		Arc::clone(&source),
		fault_common::cold_payload(),
		fault_common::cold_config(),
		CancellationToken::new(),
		tier.clone(),
	)
	.await?;

	let branch_prefix = fault_common::branch_object_prefix();
	let snapshot_key = tier
		.list_prefix(&format!("{branch_prefix}/pointer_snapshot/"))
		.await?
		.into_iter()
		.next()
		.context("pointer snapshot should exist")?
		.key;
	let snapshot = decode_pointer_snapshot(
		&tier
			.get_object(&snapshot_key)
			.await?
			.context("pointer snapshot object should exist")?,
	)?;
	let branch_record_key = format!("{branch_prefix}/branch_record.bare");
	let branch_record_bytes = tier
		.get_object(&branch_record_key)
		.await?
		.context("branch record object should exist")?;
	let branch_record = decode_database_branch_record(&branch_record_bytes)?;
	let image_key = format!("{branch_prefix}/image/00000000/00000002-0000000000000005.ltx");
	let image_bytes = tier
		.get_object(&image_key)
		.await?
		.context("image layer should exist")?;
	let snapshot_database = snapshot.databases[0].clone();
	let branch_record_bytes_for_restore = branch_record_bytes.clone();
	let image_bytes_for_restore = image_bytes.clone();

	let restored = fault_common::test_db("depot-dr-restored-").await?;
	restored
		.run(move |tx| {
			let snapshot_database = snapshot_database.clone();
			let branch_record_bytes = branch_record_bytes_for_restore.clone();
			let image_bytes = image_bytes_for_restore.clone();
			async move {
				tx.informal().set(
					&namespace_pointer_cur_key(NamespaceId::from_gas_id(Id::v1(
						uuid::Uuid::nil(),
						1,
					))),
					&encode_namespace_pointer(NamespacePointer {
						current_branch: snapshot_database.1,
						last_swapped_at_ms: 0,
					})?,
				);
				tx.informal().set(
					&database_pointer_cur_key(snapshot_database.1, &snapshot_database.0),
					&encode_database_pointer(DatabasePointer {
						current_branch: snapshot_database.2,
						last_swapped_at_ms: 0,
					})?,
				);
				tx.informal()
					.set(&branches_list_key(snapshot_database.2), &branch_record_bytes);
				tx.informal().set(
					&branch_meta_head_key(snapshot_database.2),
					&encode_db_head(DBHead {
						head_txid: 5,
						db_size_pages: 64,
						post_apply_checksum: 99,
						branch_id: snapshot_database.2,
						#[cfg(debug_assertions)]
						generation: 0,
					})?,
				);
				tx.informal()
					.set(&branch_shard_key(snapshot_database.2, 2, 5), &image_bytes);
				Ok(())
			}
		})
		.await?;

	assert_eq!(branch_record.branch_id, fault_common::database_branch_id());
	assert_eq!(
		fault_common::read_value(&restored, branches_list_key(fault_common::database_branch_id()))
			.await?,
		Some(branch_record_bytes)
	);
	assert_eq!(
		fault_common::read_value(
			&restored,
			branch_shard_key(fault_common::database_branch_id(), 2, 5)
		)
		.await?,
		Some(b"shard-five".to_vec())
	);

	Ok(())
}
