mod common;
mod fork_common;

use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use depot::{
	conveyer::branch,
	keys::{
		bucket_branches_restore_point_pin_key, bucket_catalog_by_db_key, bucket_child_key,
		bucket_fork_pin_key, bucket_proof_epoch_key,
	},
	types::{
		BucketBranchId, BucketId, ResolvedVersionstamp, decode_bucket_catalog_db_fact,
		decode_bucket_fork_fact,
	},
};
use gas::prelude::Id;
use universaldb::options::MutationType;

use fork_common::{
	assert_storage_error, page, page_bytes, read_bucket_branch_id_for, read_bucket_branch_record,
	read_commit, read_database_branch_id, read_value,
};

fn bucket_id_to_gas_id(bucket_id: BucketId) -> Id {
	Id::v1(bucket_id.as_uuid(), 1)
}

#[tokio::test]
async fn fork_bucket_covers_root_depth_one_and_deep_sources() -> Result<()> {
	common::test_matrix("depot-fork-bucket-covers", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();
			let source_bucket_gas = ctx.bucket_id;
			let source = ctx.make_db(source_bucket_gas, database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let root_database_branch =
				read_database_branch_id(&db, source_bucket_gas, &database_id).await?;
			let root_commit = read_commit(&db, root_database_branch, 1).await?;
			let mut source_bucket = BucketId::from_gas_id(source_bucket_gas);
			let mut source_bucket_branch =
				read_bucket_branch_id_for(&db, source_bucket_gas).await?;

			for depth in 1..=4 {
				let forked_bucket = branch::fork_bucket(
					&db,
					source_bucket,
					ResolvedVersionstamp {
						versionstamp: root_commit.versionstamp,
						restore_point: None,
					},
				)
				.await?;
				let forked_bucket_branch =
					read_bucket_branch_id_for(&db, bucket_id_to_gas_id(forked_bucket)).await?;
				let forked_record = read_bucket_branch_record(&db, forked_bucket_branch).await?;
				assert_eq!(forked_record.parent, Some(source_bucket_branch));
				assert_eq!(forked_record.fork_depth, depth);

				let forked_database_db =
					ctx.make_db(bucket_id_to_gas_id(forked_bucket), database_id.clone());
				let pages = forked_database_db.get_pages(vec![1]).await?;
				assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

				source_bucket = forked_bucket;
				source_bucket_branch = forked_bucket_branch;
			}

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_bucket_writes_unresolved_proof_facts() -> Result<()> {
	common::test_matrix("depot-fork-bucket-proof-facts", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();
			let source_bucket_gas = ctx.bucket_id;
			let source = ctx.make_db(source_bucket_gas, database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let root_database_branch =
				read_database_branch_id(&db, source_bucket_gas, &database_id).await?;
			let root_commit = read_commit(&db, root_database_branch, 1).await?;
			let source_bucket_branch = read_bucket_branch_id_for(&db, source_bucket_gas).await?;

			let forked_bucket = branch::fork_bucket(
				&db,
				BucketId::from_gas_id(source_bucket_gas),
				ResolvedVersionstamp {
					versionstamp: root_commit.versionstamp,
					restore_point: None,
				},
			)
			.await?;
			let forked_bucket_branch =
				read_bucket_branch_id_for(&db, bucket_id_to_gas_id(forked_bucket)).await?;

			let bucket_catalog_fact_bytes = read_value(
				&db,
				bucket_catalog_by_db_key(root_database_branch, source_bucket_branch),
			)
			.await?
			.expect("bucket catalog proof fact should exist");
			let bucket_catalog_fact = decode_bucket_catalog_db_fact(&bucket_catalog_fact_bytes)?;
			assert_eq!(bucket_catalog_fact.database_branch_id, root_database_branch);
			assert_eq!(bucket_catalog_fact.bucket_branch_id, source_bucket_branch);
			assert!(bucket_catalog_fact.catalog_versionstamp <= root_commit.versionstamp);
			assert_eq!(bucket_catalog_fact.tombstone_versionstamp, None);

			let fork_fact_bytes = read_value(
				&db,
				bucket_fork_pin_key(
					source_bucket_branch,
					root_commit.versionstamp,
					forked_bucket_branch,
				),
			)
			.await?
			.expect("bucket fork pin fact should exist");
			let child_fact_bytes = read_value(
				&db,
				bucket_child_key(
					source_bucket_branch,
					root_commit.versionstamp,
					forked_bucket_branch,
				),
			)
			.await?
			.expect("bucket child fact should exist");
			let fork_fact = decode_bucket_fork_fact(&fork_fact_bytes)?;
			let child_fact = decode_bucket_fork_fact(&child_fact_bytes)?;
			assert_eq!(fork_fact, child_fact);
			assert_eq!(fork_fact.source_bucket_branch_id, source_bucket_branch);
			assert_eq!(fork_fact.target_bucket_branch_id, forked_bucket_branch);
			assert_eq!(fork_fact.fork_versionstamp, root_commit.versionstamp);

			assert!(
				read_value(&db, bucket_proof_epoch_key(source_bucket_branch))
					.await?
					.is_some()
			);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_bucket_restore_point_pin_race_returns_out_of_retention() -> Result<()> {
	common::test_matrix("depot-fork-bucket-pin-race", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();
			let source_bucket_gas = ctx.bucket_id;
			let source = ctx.make_db(source_bucket_gas, database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let root_database_branch =
				read_database_branch_id(&db, source_bucket_gas, &database_id).await?;
			let root_commit = read_commit(&db, root_database_branch, 1).await?;
			let source_bucket_branch = read_bucket_branch_id_for(&db, source_bucket_gas).await?;
			let new_branch = BucketBranchId::new_v4();
			let pin_after_fork_point = [0xff; 16];
			let raced = Arc::new(AtomicBool::new(false));

			let err = db
				.run({
					let db = db.clone();
					let raced = raced.clone();
					move |tx| {
						let db = db.clone();
						let raced = raced.clone();
						async move {
							branch::derive_bucket_branch_at(
								&tx,
								source_bucket_branch,
								root_commit.versionstamp,
								new_branch,
								None,
							)
							.await?;

							if !raced.swap(true, Ordering::SeqCst) {
								db.run(move |pin_tx| async move {
									pin_tx.informal().atomic_op(
										&bucket_branches_restore_point_pin_key(
											source_bucket_branch,
										),
										&pin_after_fork_point,
										MutationType::ByteMin,
									);
									Ok(())
								})
								.await?;
							}

							Ok(())
						}
					}
				})
				.await
				.expect_err("retry should observe the advanced bucket restore_point pin");

			assert_storage_error(&err, depot::error::SqliteStorageError::ForkOutOfRetention);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_bucket_allows_depth_sixteen_and_rejects_depth_seventeen() -> Result<()> {
	common::test_matrix("depot-fork-bucket-depth", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();
			let source_bucket_gas = ctx.bucket_id;
			let source = ctx.make_db(source_bucket_gas, database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let root_database_branch =
				read_database_branch_id(&db, source_bucket_gas, &database_id).await?;
			let root_commit = read_commit(&db, root_database_branch, 1).await?;
			let mut source_bucket = BucketId::from_gas_id(source_bucket_gas);
			let mut source_bucket_branch =
				read_bucket_branch_id_for(&db, source_bucket_gas).await?;

			for depth in 1..=depot::constants::MAX_BUCKET_DEPTH {
				let forked_bucket = branch::fork_bucket(
					&db,
					source_bucket,
					ResolvedVersionstamp {
						versionstamp: root_commit.versionstamp,
						restore_point: None,
					},
				)
				.await?;
				let forked_bucket_branch =
					read_bucket_branch_id_for(&db, bucket_id_to_gas_id(forked_bucket)).await?;
				let forked_record = read_bucket_branch_record(&db, forked_bucket_branch).await?;
				assert_eq!(forked_record.parent, Some(source_bucket_branch));
				assert_eq!(forked_record.fork_depth, depth);

				source_bucket = forked_bucket;
				source_bucket_branch = forked_bucket_branch;
			}

			let err = branch::fork_bucket(
				&db,
				source_bucket,
				ResolvedVersionstamp {
					versionstamp: root_commit.versionstamp,
					restore_point: None,
				},
			)
			.await
			.expect_err("depth 17 bucket fork should be rejected");

			assert_storage_error(
				&err,
				depot::error::SqliteStorageError::BucketForkChainTooDeep,
			);
			assert_eq!(
				read_bucket_branch_record(&db, source_bucket_branch)
					.await?
					.fork_depth,
				depot::constants::MAX_BUCKET_DEPTH
			);

			Ok(())
		})
	})
	.await
}
