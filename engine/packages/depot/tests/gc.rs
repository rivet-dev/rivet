mod common;

use anyhow::Result;
use depot::{
	conveyer::history_pin,
	gc::{
		VERSIONSTAMP_INFINITY, estimate_branch_gc_pin, sweep_branch_hot_history,
		sweep_unreferenced_branch,
	},
	keys::{
		branch_commit_key, branch_delta_chunk_key, branch_meta_head_key, branch_pidx_key,
		branch_shard_key, branch_vtx_key, branches_desc_pin_key, branches_list_key,
		branches_refcount_key, branches_restore_point_pin_key, db_pin_key,
	},
	types::{
		BranchState, BucketBranchId, CommitRow, DatabaseBranchId, DatabaseBranchRecord,
		encode_commit_row, encode_database_branch_record,
	},
};
use universaldb::utils::IsolationLevel::Snapshot;

fn branch_id() -> DatabaseBranchId {
	DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(
		0x1234_5678_9abc_def0_0123_4567_89ab_cdef,
	))
}

async fn read_value(db: &universaldb::Database, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
	db.run(move |tx| {
		let key = key.clone();
		async move {
			Ok(tx
				.informal()
				.get(&key, Snapshot)
				.await?
				.map(Vec::<u8>::from))
		}
	})
	.await
}

async fn read_refcount(db: &universaldb::Database, branch_id: DatabaseBranchId) -> Result<i64> {
	let bytes = read_value(db, branches_refcount_key(branch_id))
		.await?
		.expect("branch refcount should exist");
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.expect("branch refcount should be i64 LE");

	Ok(i64::from_le_bytes(bytes))
}

async fn seed_branch(
	db: &universaldb::Database,
	refcount: i64,
	root_versionstamp: [u8; 16],
) -> Result<()> {
	let branch_id = branch_id();
	db.run(move |tx| async move {
		tx.informal().set(
			&branches_list_key(branch_id),
			&encode_database_branch_record(DatabaseBranchRecord {
				branch_id,
				bucket_branch: BucketBranchId::nil(),
				parent: None,
				parent_versionstamp: None,
				root_versionstamp,
				fork_depth: 0,
				created_at_ms: 1_000,
				created_from_restore_point: None,
				state: BranchState::Live,
				lifecycle_generation: 0,
			})?,
		);
		tx.informal()
			.set(&branches_refcount_key(branch_id), &refcount.to_le_bytes());
		Ok(())
	})
	.await
}

async fn write_commit(db: &universaldb::Database, txid: u64, versionstamp: [u8; 16]) -> Result<()> {
	let branch_id = branch_id();
	db.run(move |tx| async move {
		tx.informal().set(
			&branch_commit_key(branch_id, txid),
			&encode_commit_row(CommitRow {
				wall_clock_ms: i64::try_from(txid).expect("test txid should fit i64"),
				versionstamp,
				db_size_pages: 1,
				post_apply_checksum: txid,
			})?,
		);
		tx.informal().set(
			&branch_vtx_key(branch_id, versionstamp),
			&txid.to_be_bytes(),
		);
		Ok(())
	})
	.await
}

#[tokio::test]
async fn sweeping_child_branch_releases_parent_refcount_and_fork_pin() -> Result<()> {
	common::test_matrix("depot-gc-child-branch", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let parent_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(
				0xaaaa_0000_0000_0000_0000_0000_0000_0001,
			));
			let child_branch_id = DatabaseBranchId::from_uuid(uuid::Uuid::from_u128(
				0xbbbb_0000_0000_0000_0000_0000_0000_0002,
			));
			let fork_versionstamp = [2; 16];

			db.run(move |tx| async move {
				tx.informal().set(
					&branches_list_key(parent_branch_id),
					&encode_database_branch_record(DatabaseBranchRecord {
						branch_id: parent_branch_id,
						bucket_branch: BucketBranchId::nil(),
						parent: None,
						parent_versionstamp: None,
						root_versionstamp: [1; 16],
						fork_depth: 0,
						created_at_ms: 1_000,
						created_from_restore_point: None,
						state: BranchState::Live,
						lifecycle_generation: 0,
					})?,
				);
				tx.informal().set(
					&branches_refcount_key(parent_branch_id),
					&2_i64.to_le_bytes(),
				);
				tx.informal()
					.set(&branches_desc_pin_key(parent_branch_id), &fork_versionstamp);

				tx.informal().set(
					&branches_list_key(child_branch_id),
					&encode_database_branch_record(DatabaseBranchRecord {
						branch_id: child_branch_id,
						bucket_branch: BucketBranchId::nil(),
						parent: Some(parent_branch_id),
						parent_versionstamp: Some(fork_versionstamp),
						root_versionstamp: fork_versionstamp,
						fork_depth: 1,
						created_at_ms: 1_001,
						created_from_restore_point: None,
						state: BranchState::Live,
						lifecycle_generation: 0,
					})?,
				);
				tx.informal().set(
					&branches_refcount_key(child_branch_id),
					&0_i64.to_le_bytes(),
				);
				history_pin::write_database_fork_pin(
					&tx,
					parent_branch_id,
					child_branch_id,
					fork_versionstamp,
					1,
					1_001,
				)?;

				Ok(())
			})
			.await?;

			let outcome = sweep_unreferenced_branch(&db, child_branch_id)
				.await?
				.expect("child branch should be swept");
			assert!(outcome.branch_deleted);

			assert_eq!(read_refcount(&db, parent_branch_id).await?, 1);
			assert_eq!(
				read_value(
					&db,
					db_pin_key(
						parent_branch_id,
						&history_pin::database_fork_pin_id(child_branch_id),
					),
				)
				.await?,
				None
			);
			assert_eq!(
				read_value(&db, branches_desc_pin_key(parent_branch_id)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branches_list_key(child_branch_id)).await?,
				None
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn branch_gc_pin_recomputes_from_current_counters_without_ratchet() -> Result<()> {
	common::test_matrix("depot-gc-pin-recompute", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = branch_id();
			seed_branch(&db, 1, [10; 16]).await?;

			db.run(move |tx| async move {
				tx.informal()
					.set(&branches_desc_pin_key(branch_id), &[4; 16]);
				tx.informal()
					.set(&branches_restore_point_pin_key(branch_id), &[8; 16]);
				Ok(())
			})
			.await?;
			let pin = estimate_branch_gc_pin(&db, branch_id)
				.await?
				.expect("branch should have a GC pin");
			assert_eq!(pin.gc_pin, [4; 16]);

			db.run(move |tx| async move {
				tx.informal()
					.set(&branches_desc_pin_key(branch_id), &VERSIONSTAMP_INFINITY);
				Ok(())
			})
			.await?;
			let pin = estimate_branch_gc_pin(&db, branch_id)
				.await?
				.expect("branch should have a GC pin");
			assert_eq!(pin.gc_pin, [8; 16]);

			db.run(move |tx| async move {
				tx.informal()
					.set(&branches_refcount_key(branch_id), &0_i64.to_le_bytes());
				tx.informal().set(
					&branches_restore_point_pin_key(branch_id),
					&VERSIONSTAMP_INFINITY,
				);
				Ok(())
			})
			.await?;
			let pin = estimate_branch_gc_pin(&db, branch_id)
				.await?
				.expect("branch should have a GC pin");
			assert_eq!(pin.gc_pin, VERSIONSTAMP_INFINITY);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn unreferenced_unpinned_branch_sweep_deletes_hot_branch_data() -> Result<()> {
	common::test_matrix("depot-gc-unreferenced-sweep", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = branch_id();
			seed_branch(&db, 0, [6; 16]).await?;
			write_commit(&db, 3, [3; 16]).await?;
			write_commit(&db, 6, [6; 16]).await?;

			db.run(move |tx| async move {
				tx.informal()
					.set(&branches_desc_pin_key(branch_id), &VERSIONSTAMP_INFINITY);
				tx.informal().set(
					&branches_restore_point_pin_key(branch_id),
					&VERSIONSTAMP_INFINITY,
				);
				tx.informal().set(&branch_meta_head_key(branch_id), b"head");
				tx.informal()
					.set(&branch_pidx_key(branch_id, 7), &6_u64.to_be_bytes());
				tx.informal()
					.set(&branch_delta_chunk_key(branch_id, 3, 0), b"delta-three");
				tx.informal()
					.set(&branch_shard_key(branch_id, 0, 6), b"shard-six");
				Ok(())
			})
			.await?;

			let outcome = sweep_unreferenced_branch(&db, branch_id)
				.await?
				.expect("branch should be swept");
			assert!(outcome.branch_deleted);
			assert_eq!(outcome.gc_pin, VERSIONSTAMP_INFINITY);
			assert!(outcome.keys_deleted >= 9);

			assert_eq!(read_value(&db, branches_list_key(branch_id)).await?, None);
			assert_eq!(
				read_value(&db, branches_refcount_key(branch_id)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branches_desc_pin_key(branch_id)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branches_restore_point_pin_key(branch_id)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_meta_head_key(branch_id)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_commit_key(branch_id, 3)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_commit_key(branch_id, 6)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_vtx_key(branch_id, [3; 16])).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_vtx_key(branch_id, [6; 16])).await?,
				None
			);
			assert_eq!(read_value(&db, branch_pidx_key(branch_id, 7)).await?, None);
			assert_eq!(
				read_value(&db, branch_delta_chunk_key(branch_id, 3, 0)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_shard_key(branch_id, 0, 6)).await?,
				None
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn branch_hot_gc_uses_vtx_floor_for_commits_vtx_and_delta() -> Result<()> {
	common::test_matrix("depot-gc-hot-history", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let branch_id = branch_id();
			seed_branch(&db, 1, [6; 16]).await?;
			write_commit(&db, 3, [3; 16]).await?;
			write_commit(&db, 4, [4; 16]).await?;
			write_commit(&db, 6, [6; 16]).await?;
			write_commit(&db, 8, [8; 16]).await?;

			db.run(move |tx| async move {
				tx.informal()
					.set(&branch_delta_chunk_key(branch_id, 2, 0), b"delta-two");
				tx.informal()
					.set(&branch_delta_chunk_key(branch_id, 5, 0), b"delta-five");
				tx.informal()
					.set(&branch_delta_chunk_key(branch_id, 6, 0), b"delta-six");
				Ok(())
			})
			.await?;

			let outcome = sweep_branch_hot_history(&db, branch_id)
				.await?
				.expect("branch should be swept");
			assert_eq!(outcome.gc_pin, [6; 16]);
			assert_eq!(outcome.txid_floor, Some(6));
			assert_eq!(outcome.commits_deleted, 2);
			assert_eq!(outcome.vtx_deleted, 2);
			assert_eq!(outcome.delta_chunks_deleted, 2);

			assert_eq!(
				read_value(&db, branch_commit_key(branch_id, 3)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_commit_key(branch_id, 4)).await?,
				None
			);
			assert!(
				read_value(&db, branch_commit_key(branch_id, 6))
					.await?
					.is_some()
			);
			assert!(
				read_value(&db, branch_commit_key(branch_id, 8))
					.await?
					.is_some()
			);
			assert_eq!(
				read_value(&db, branch_vtx_key(branch_id, [3; 16])).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_vtx_key(branch_id, [4; 16])).await?,
				None
			);
			assert!(
				read_value(&db, branch_vtx_key(branch_id, [6; 16]))
					.await?
					.is_some()
			);
			assert!(
				read_value(&db, branch_vtx_key(branch_id, [8; 16]))
					.await?
					.is_some()
			);
			assert_eq!(
				read_value(&db, branch_delta_chunk_key(branch_id, 2, 0)).await?,
				None
			);
			assert_eq!(
				read_value(&db, branch_delta_chunk_key(branch_id, 5, 0)).await?,
				None
			);
			assert!(
				read_value(&db, branch_delta_chunk_key(branch_id, 6, 0))
					.await?
					.is_some()
			);

			Ok(())
		})
	})
	.await
}
