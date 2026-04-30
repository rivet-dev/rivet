mod fork_common;

use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use sqlite_storage::{
	keys::branches_bk_pin_key,
	pump::branch,
	types::{ActorBranchId, NamespaceId, ResolvedVersionstamp},
};
use universaldb::options::MutationType;

use fork_common::{
	TEST_ACTOR, actor_db, assert_storage_error, page, page_bytes, read_actor_branch_id,
	read_actor_branch_record, read_commit, read_head_commit, read_namespace_branch_id_for,
	target_namespace, test_db, test_namespace, test_ups,
};

#[tokio::test]
async fn fork_actor_covers_root_depth_one_deep_and_cross_namespace_sources() -> Result<()> {
	let db = test_db().await?;
	let source = actor_db(db.clone(), test_namespace(), TEST_ACTOR);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let root_branch = read_actor_branch_id(&db, test_namespace(), TEST_ACTOR).await?;
	let root_commit = read_commit(&db, root_branch, 1).await?;

	let target_seed = actor_db(db.clone(), target_namespace(), "target-seed");
	target_seed.commit(vec![page(1, 0xaa)], 1, 1_100).await?;
	let target_namespace_branch = read_namespace_branch_id_for(&db, target_namespace()).await?;

	let cross_namespace_actor = branch::fork_actor(
		&db,
		&test_ups(),
		NamespaceId::from_gas_id(test_namespace()),
		TEST_ACTOR.to_string(),
		ResolvedVersionstamp {
			versionstamp: root_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(target_namespace()),
	)
	.await?;
	let cross_namespace_branch =
		read_actor_branch_id(&db, target_namespace(), &cross_namespace_actor).await?;
	let cross_namespace_record = read_actor_branch_record(&db, cross_namespace_branch).await?;
	assert_eq!(cross_namespace_record.namespace_branch, target_namespace_branch);
	assert_eq!(cross_namespace_record.parent, Some(root_branch));
	assert_eq!(cross_namespace_record.fork_depth, 1);
	let cross_namespace_db = actor_db(db.clone(), target_namespace(), cross_namespace_actor);
	let pages = cross_namespace_db.get_pages(vec![1]).await?;
	assert_eq!(pages[0].bytes, Some(page_bytes(0x11)));

	let mut source_actor_id = TEST_ACTOR.to_string();
	let mut source_commit = root_commit;
	let mut expected_pages = vec![(1, 0x11)];

	for depth in 1..=4 {
		let forked_actor_id = branch::fork_actor(
			&db,
			&test_ups(),
			NamespaceId::from_gas_id(test_namespace()),
			source_actor_id.clone(),
			ResolvedVersionstamp {
				versionstamp: source_commit.versionstamp,
				bookmark: None,
			},
			NamespaceId::from_gas_id(test_namespace()),
		)
		.await?;
		let forked_branch = read_actor_branch_id(&db, test_namespace(), &forked_actor_id).await?;
		let forked_record = read_actor_branch_record(&db, forked_branch).await?;
		assert_eq!(forked_record.fork_depth, depth);

		let forked_db = actor_db(db.clone(), test_namespace(), forked_actor_id.clone());
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
			source_actor_id = forked_actor_id;
		}
	}

	Ok(())
}

#[tokio::test]
async fn fork_actor_bk_pin_race_returns_out_of_retention() -> Result<()> {
	let db = test_db().await?;
	let source = actor_db(db.clone(), test_namespace(), TEST_ACTOR);
	source.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let source_branch = read_actor_branch_id(&db, test_namespace(), TEST_ACTOR).await?;
	let namespace_branch = read_namespace_branch_id_for(&db, test_namespace()).await?;
	let source_commit = read_commit(&db, source_branch, 1).await?;
	let new_branch = ActorBranchId::new_v4();
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
						namespace_branch,
						None,
					)
					.await?;

					if !raced.swap(true, Ordering::SeqCst) {
						db.run(move |pin_tx| async move {
							pin_tx.informal().atomic_op(
								&branches_bk_pin_key(source_branch),
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
		.expect_err("retry should observe the advanced bookmark pin");

	assert_storage_error(&err, sqlite_storage::error::SqliteStorageError::ForkOutOfRetention);

	Ok(())
}

#[tokio::test]
async fn fork_actor_allows_depth_sixteen_and_rejects_depth_seventeen() -> Result<()> {
	let db = test_db().await?;
	let root = actor_db(db.clone(), test_namespace(), TEST_ACTOR);
	root.commit(vec![page(1, 0x11)], 2, 1_000).await?;
	let mut source_actor_id = TEST_ACTOR.to_string();
	let mut source_branch = read_actor_branch_id(&db, test_namespace(), TEST_ACTOR).await?;
	let mut source_commit = read_commit(&db, source_branch, 1).await?;

	for depth in 1..=sqlite_storage::constants::MAX_FORK_DEPTH {
		let forked_actor_id = branch::fork_actor(
			&db,
			&test_ups(),
			NamespaceId::from_gas_id(test_namespace()),
			source_actor_id.clone(),
			ResolvedVersionstamp {
				versionstamp: source_commit.versionstamp,
				bookmark: None,
			},
			NamespaceId::from_gas_id(test_namespace()),
		)
		.await?;
		let forked_branch = read_actor_branch_id(&db, test_namespace(), &forked_actor_id).await?;
		let forked_record = read_actor_branch_record(&db, forked_branch).await?;
		assert_eq!(forked_record.fork_depth, depth);

		if depth < sqlite_storage::constants::MAX_FORK_DEPTH {
			let forked_db = actor_db(db.clone(), test_namespace(), forked_actor_id.clone());
			let pgno = depth as u32 + 1;
			forked_db
				.commit(vec![page(pgno, 0x30 + depth)], pgno + 1, 3_000 + depth as i64)
				.await?;
			source_commit = read_head_commit(&db, forked_branch).await?;
			source_actor_id = forked_actor_id;
			source_branch = forked_branch;
		} else {
			source_actor_id = forked_actor_id;
			source_branch = forked_branch;
		}
	}

	let err = branch::fork_actor(
		&db,
		&test_ups(),
		NamespaceId::from_gas_id(test_namespace()),
		source_actor_id,
		ResolvedVersionstamp {
			versionstamp: source_commit.versionstamp,
			bookmark: None,
		},
		NamespaceId::from_gas_id(test_namespace()),
	)
	.await
	.expect_err("depth 17 fork should be rejected");

	assert_storage_error(&err, sqlite_storage::error::SqliteStorageError::ForkChainTooDeep);
	assert_eq!(
		read_actor_branch_record(&db, source_branch).await?.fork_depth,
		sqlite_storage::constants::MAX_FORK_DEPTH
	);

	Ok(())
}
