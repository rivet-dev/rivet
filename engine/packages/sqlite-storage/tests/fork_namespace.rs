mod fork_common;

use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use gas::prelude::Id;
use sqlite_storage::{
	keys::namespace_branches_bk_pin_key,
	pump::branch,
	types::{NamespaceBranchId, NamespaceId, ResolvedVersionstamp},
};
use universaldb::options::MutationType;

use fork_common::{
	TEST_DATABASE, make_db, assert_storage_error, page, page_bytes, read_database_branch_id,
	read_commit, read_namespace_branch_id_for, read_namespace_branch_record, test_db,
	test_namespace, test_ups,
};

fn namespace_id_to_gas_id(namespace_id: NamespaceId) -> Id {
	Id::v1(namespace_id.as_uuid(), 1)
}

#[tokio::test]
async fn fork_namespace_covers_root_depth_one_and_deep_sources() -> Result<()> {
	let db = test_db().await?;
	let source = make_db(db.clone(), test_namespace(), TEST_DATABASE);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let root_database_branch = read_database_branch_id(&db, test_namespace(), TEST_DATABASE).await?;
	let root_commit = read_commit(&db, root_database_branch, 1).await?;
	let mut source_namespace = NamespaceId::from_gas_id(test_namespace());
	let mut source_namespace_branch = read_namespace_branch_id_for(&db, test_namespace()).await?;

	for depth in 1..=4 {
		let forked_namespace = branch::fork_namespace(
			&db,
			&test_ups(),
			source_namespace,
			ResolvedVersionstamp {
				versionstamp: root_commit.versionstamp,
				bookmark: None,
			},
		)
		.await?;
		let forked_namespace_branch =
			read_namespace_branch_id_for(&db, namespace_id_to_gas_id(forked_namespace)).await?;
		let forked_record = read_namespace_branch_record(&db, forked_namespace_branch).await?;
		assert_eq!(forked_record.parent, Some(source_namespace_branch));
		assert_eq!(forked_record.fork_depth, depth);

		let forked_database_db = make_db(
			db.clone(),
			namespace_id_to_gas_id(forked_namespace),
			TEST_DATABASE,
		);
		let pages = forked_database_db.get_pages(vec![1]).await?;
		assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

		source_namespace = forked_namespace;
		source_namespace_branch = forked_namespace_branch;
	}

	Ok(())
}

#[tokio::test]
async fn fork_namespace_bk_pin_race_returns_out_of_retention() -> Result<()> {
	let db = test_db().await?;
	let source = make_db(db.clone(), test_namespace(), TEST_DATABASE);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let root_database_branch = read_database_branch_id(&db, test_namespace(), TEST_DATABASE).await?;
	let root_commit = read_commit(&db, root_database_branch, 1).await?;
	let source_namespace_branch = read_namespace_branch_id_for(&db, test_namespace()).await?;
	let new_branch = NamespaceBranchId::new_v4();
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
					branch::derive_namespace_branch_at(
						&tx,
						source_namespace_branch,
						root_commit.versionstamp,
						new_branch,
						None,
					)
					.await?;

					if !raced.swap(true, Ordering::SeqCst) {
						db.run(move |pin_tx| async move {
							pin_tx.informal().atomic_op(
								&namespace_branches_bk_pin_key(source_namespace_branch),
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
		.expect_err("retry should observe the advanced namespace bookmark pin");

	assert_storage_error(&err, sqlite_storage::error::SqliteStorageError::ForkOutOfRetention);

	Ok(())
}

#[tokio::test]
async fn fork_namespace_allows_depth_sixteen_and_rejects_depth_seventeen() -> Result<()> {
	let db = test_db().await?;
	let source = make_db(db.clone(), test_namespace(), TEST_DATABASE);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let root_database_branch = read_database_branch_id(&db, test_namespace(), TEST_DATABASE).await?;
	let root_commit = read_commit(&db, root_database_branch, 1).await?;
	let mut source_namespace = NamespaceId::from_gas_id(test_namespace());
	let mut source_namespace_branch = read_namespace_branch_id_for(&db, test_namespace()).await?;

	for depth in 1..=sqlite_storage::constants::MAX_NAMESPACE_DEPTH {
		let forked_namespace = branch::fork_namespace(
			&db,
			&test_ups(),
			source_namespace,
			ResolvedVersionstamp {
				versionstamp: root_commit.versionstamp,
				bookmark: None,
			},
		)
		.await?;
		let forked_namespace_branch =
			read_namespace_branch_id_for(&db, namespace_id_to_gas_id(forked_namespace)).await?;
		let forked_record = read_namespace_branch_record(&db, forked_namespace_branch).await?;
		assert_eq!(forked_record.parent, Some(source_namespace_branch));
		assert_eq!(forked_record.fork_depth, depth);

		source_namespace = forked_namespace;
		source_namespace_branch = forked_namespace_branch;
	}

	let err = branch::fork_namespace(
		&db,
		&test_ups(),
		source_namespace,
		ResolvedVersionstamp {
			versionstamp: root_commit.versionstamp,
			bookmark: None,
		},
	)
	.await
	.expect_err("depth 17 namespace fork should be rejected");

	assert_storage_error(
		&err,
		sqlite_storage::error::SqliteStorageError::NamespaceForkChainTooDeep,
	);
	assert_eq!(
		read_namespace_branch_record(&db, source_namespace_branch)
			.await?
			.fork_depth,
		sqlite_storage::constants::MAX_NAMESPACE_DEPTH
	);

	Ok(())
}
