mod common;
mod fork_common;

use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use depot::{
	conveyer::branch,
	keys::{branch_meta_head_at_fork_key, branches_restore_point_pin_key},
	pitr_interval::write_pitr_interval_coverage,
	types::{
		BucketId, DatabaseBranchId, PitrIntervalCoverage, ResolvedVersionstamp, SnapshotSelector,
		decode_db_head,
	},
};
use universaldb::options::MutationType;

use fork_common::{
	assert_storage_error, page, page_bytes, read_bucket_branch_id_for, read_commit,
	read_database_branch_id, read_database_branch_record, read_head_commit, read_value,
	target_bucket,
};

#[tokio::test]
async fn fork_database_covers_root_depth_one_deep_and_cross_bucket_sources() -> Result<()> {
	common::test_matrix("depot-fork-database-covers", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let root_database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, root_database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let root_branch =
				read_database_branch_id(&db, source_bucket, &root_database_id).await?;
			let root_commit = read_commit(&db, root_branch, 1).await?;

			let target_seed = ctx.make_db(target_bucket(), "target-seed");
			target_seed.commit(vec![page(1, 0xaa)], 1, 1_100).await?;
			let target_bucket_branch = read_bucket_branch_id_for(&db, target_bucket()).await?;

			let cross_bucket_database = branch::fork_database(
				&db,
				BucketId::from_gas_id(source_bucket),
				root_database_id.clone(),
				SnapshotSelector::Latest,
				BucketId::from_gas_id(target_bucket()),
			)
			.await?;
			let cross_bucket_branch =
				read_database_branch_id(&db, target_bucket(), &cross_bucket_database).await?;
			let cross_bucket_record = read_database_branch_record(&db, cross_bucket_branch).await?;
			assert_eq!(cross_bucket_record.bucket_branch, target_bucket_branch);
			assert_eq!(cross_bucket_record.parent, Some(root_branch));
			assert_eq!(cross_bucket_record.fork_depth, 1);
			let cross_bucket_db = ctx.make_db(target_bucket(), cross_bucket_database);
			let pages = cross_bucket_db.get_pages(vec![1]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

			let mut source_database_id = root_database_id;
			let mut source_commit = root_commit;
			let mut expected_pages = vec![(1, 0x11)];

			for depth in 1..=4 {
				let forked_database_id = branch::fork_database(
					&db,
					BucketId::from_gas_id(source_bucket),
					source_database_id.clone(),
					ResolvedVersionstamp {
						versionstamp: source_commit.versionstamp,
						restore_point: None,
					},
					BucketId::from_gas_id(source_bucket),
				)
				.await?;
				let forked_branch =
					read_database_branch_id(&db, source_bucket, &forked_database_id).await?;
				let forked_record = read_database_branch_record(&db, forked_branch).await?;
				assert_eq!(forked_record.fork_depth, depth);

				let forked_db = ctx.make_db(source_bucket, forked_database_id.clone());
				let pgnos = expected_pages.iter().map(|(pgno, _)| *pgno).collect();
				let pages = forked_db.get_pages(pgnos).await?;
				for (page, (_, fill)) in pages.iter().zip(expected_pages.iter()) {
					assert_eq!(page.bytes, Some(page_bytes(*fill)));
				}

				if depth < 4 {
					let pgno = depth as u32 + 1;
					let fill = 0x20 + depth;
					forked_db
						.commit(vec![page(pgno, fill)], pgno + 1, 2_000 + depth as i64)
						.await?;
					expected_pages.push((pgno, fill));
					source_commit = read_head_commit(&db, forked_branch).await?;
					source_database_id = forked_database_id;
				}
			}

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_database_from_timestamp_selector_uses_resolved_snapshot() -> Result<()> {
	common::test_matrix("depot-fork-database-timestamp", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let source_database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, source_database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			source.commit(vec![page(1, 0x22)], 2, 2_000).await?;
			let source_branch =
				read_database_branch_id(&db, source_bucket, &source_database_id).await?;
			let first = read_commit(&db, source_branch, 1).await?;
			db.run(move |tx| async move {
				write_pitr_interval_coverage(
					&tx,
					source_branch,
					1_000,
					PitrIntervalCoverage {
						txid: 1,
						versionstamp: first.versionstamp,
						wall_clock_ms: 1_000,
						expires_at_ms: i64::MAX,
					},
				)?;
				Ok(())
			})
			.await?;

			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(source_bucket),
				source_database_id,
				SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_500,
				},
				BucketId::from_gas_id(source_bucket),
			)
			.await?;
			let forked_branch =
				read_database_branch_id(&db, source_bucket, &forked_database_id).await?;
			let head_at_fork = read_value(&db, branch_meta_head_at_fork_key(forked_branch))
				.await?
				.expect("forked branch head_at_fork should exist");
			assert_eq!(decode_db_head(&head_at_fork)?.head_txid, 1);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_database_from_restore_point_selector_uses_retained_snapshot() -> Result<()> {
	common::test_matrix("depot-fork-database-restore-point", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let source_database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, source_database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let restore_point = source
				.create_restore_point(SnapshotSelector::Latest)
				.await?;
			source.commit(vec![page(1, 0x22)], 2, 2_000).await?;

			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(source_bucket),
				source_database_id,
				SnapshotSelector::RestorePoint { restore_point },
				BucketId::from_gas_id(source_bucket),
			)
			.await?;
			let forked_branch =
				read_database_branch_id(&db, source_bucket, &forked_database_id).await?;
			let head_at_fork = read_value(&db, branch_meta_head_at_fork_key(forked_branch))
				.await?
				.expect("forked branch head_at_fork should exist");
			assert_eq!(decode_db_head(&head_at_fork)?.head_txid, 1);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_database_immediate_reopen_isolated_from_parent_later_writes() -> Result<()> {
	common::test_matrix("depot-fork-database-immediate-reopen", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let source_database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, source_database_id.clone());
			source
				.commit(vec![page(1, 0x11), page(2, 0x22)], 3, 1_000)
				.await?;
			let source_branch =
				read_database_branch_id(&db, source_bucket, &source_database_id).await?;
			let fork_point = read_commit(&db, source_branch, 1).await?;

			let forked_database_id = branch::fork_database(
				&db,
				BucketId::from_gas_id(source_bucket),
				source_database_id.clone(),
				ResolvedVersionstamp {
					versionstamp: fork_point.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(source_bucket),
			)
			.await?;
			let forked_reopen = ctx.make_db(source_bucket, forked_database_id.clone());
			let pages = forked_reopen.get_pages(vec![1, 2, 3, 4]).await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
			assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));
			assert_eq!(
				pages[2].bytes,
				Some(vec![0; depot::keys::PAGE_SIZE as usize])
			);
			assert_eq!(pages[3].bytes, None);

			source
				.commit(vec![page(1, 0x33), page(3, 0x44), page(4, 0x55)], 5, 2_000)
				.await?;
			let parent_pages = source.get_pages(vec![1, 3, 4]).await?;
			assert_eq!(parent_pages[0].bytes, Some(page_bytes(0x33)));
			assert_eq!(parent_pages[1].bytes, Some(page_bytes(0x44)));
			assert_eq!(parent_pages[2].bytes, Some(page_bytes(0x55)));

			let forked_reopen_after_parent_write = ctx.make_db(source_bucket, forked_database_id);
			let pages = forked_reopen_after_parent_write
				.get_pages(vec![1, 2, 3, 4])
				.await?;
			assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));
			assert_eq!(pages[1].bytes, Some(page_bytes(0x22)));
			assert_eq!(
				pages[2].bytes,
				Some(vec![0; depot::keys::PAGE_SIZE as usize])
			);
			assert_eq!(pages[3].bytes, None);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_database_rejects_expired_timestamp_without_target_writes() -> Result<()> {
	common::test_matrix("depot-fork-database-expired", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let source_database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, source_database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let target_seed = ctx.make_db(target_bucket(), "target-seed");
			target_seed.commit(vec![page(1, 0xaa)], 1, 1_100).await?;
			let target_bucket_id = BucketId::from_gas_id(target_bucket());
			let before = branch::list_databases(&db, target_bucket_id).await?;

			let err = branch::fork_database(
				&db,
				BucketId::from_gas_id(source_bucket),
				source_database_id,
				SnapshotSelector::AtTimestamp {
					timestamp_ms: 1_500,
				},
				target_bucket_id,
			)
			.await
			.expect_err("expired selector should reject the fork");

			assert_storage_error(&err, depot::error::SqliteStorageError::RestoreTargetExpired);
			assert_eq!(branch::list_databases(&db, target_bucket_id).await?, before);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_database_restore_point_pin_race_returns_out_of_retention() -> Result<()> {
	common::test_matrix("depot-fork-database-pin-race", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let source_database_id = ctx.database_id.clone();
			let source = ctx.make_db(source_bucket, source_database_id.clone());
			source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let source_branch =
				read_database_branch_id(&db, source_bucket, &source_database_id).await?;
			let bucket_branch = read_bucket_branch_id_for(&db, source_bucket).await?;
			let source_commit = read_commit(&db, source_branch, 1).await?;
			let new_branch = DatabaseBranchId::new_v4();
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
							branch::derive_branch_at(
								&tx,
								source_branch,
								source_commit.versionstamp,
								new_branch,
								bucket_branch,
								None,
							)
							.await?;

							if !raced.swap(true, Ordering::SeqCst) {
								db.run(move |pin_tx| async move {
									pin_tx.informal().atomic_op(
										&branches_restore_point_pin_key(source_branch),
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
				.expect_err("retry should observe the advanced restore_point pin");

			assert_storage_error(&err, depot::error::SqliteStorageError::ForkOutOfRetention);

			Ok(())
		})
	})
	.await
}

#[tokio::test]
async fn fork_database_allows_depth_sixteen_and_rejects_depth_seventeen() -> Result<()> {
	common::test_matrix("depot-fork-database-depth", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let source_bucket = ctx.bucket_id;
			let mut source_database_id = ctx.database_id.clone();
			let root = ctx.make_db(source_bucket, source_database_id.clone());
			root.commit(vec![page(1, 0x11)], 2, 1_000).await?;
			let mut source_branch =
				read_database_branch_id(&db, source_bucket, &source_database_id).await?;
			let mut source_commit = read_commit(&db, source_branch, 1).await?;

			for depth in 1..=depot::constants::MAX_FORK_DEPTH {
				let forked_database_id = branch::fork_database(
					&db,
					BucketId::from_gas_id(source_bucket),
					source_database_id.clone(),
					ResolvedVersionstamp {
						versionstamp: source_commit.versionstamp,
						restore_point: None,
					},
					BucketId::from_gas_id(source_bucket),
				)
				.await?;
				let forked_branch =
					read_database_branch_id(&db, source_bucket, &forked_database_id).await?;
				let forked_record = read_database_branch_record(&db, forked_branch).await?;
				assert_eq!(forked_record.fork_depth, depth);

				if depth < depot::constants::MAX_FORK_DEPTH {
					let forked_db = ctx.make_db(source_bucket, forked_database_id.clone());
					let pgno = depth as u32 + 1;
					forked_db
						.commit(
							vec![page(pgno, 0x30 + depth)],
							pgno + 1,
							3_000 + depth as i64,
						)
						.await?;
					source_commit = read_head_commit(&db, forked_branch).await?;
					source_database_id = forked_database_id;
					source_branch = forked_branch;
				} else {
					source_database_id = forked_database_id;
					source_branch = forked_branch;
				}
			}

			let err = branch::fork_database(
				&db,
				BucketId::from_gas_id(source_bucket),
				source_database_id,
				ResolvedVersionstamp {
					versionstamp: source_commit.versionstamp,
					restore_point: None,
				},
				BucketId::from_gas_id(source_bucket),
			)
			.await
			.expect_err("depth 17 fork should be rejected");

			assert_storage_error(&err, depot::error::SqliteStorageError::ForkChainTooDeep);
			assert_eq!(
				read_database_branch_record(&db, source_branch)
					.await?
					.fork_depth,
				depot::constants::MAX_FORK_DEPTH
			);

			Ok(())
		})
	})
	.await
}
