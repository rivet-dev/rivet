mod common;

use std::time::Duration;

use anyhow::{Context, Result};
use depot::{
	conveyer::Db,
	keys::{delta_chunk_key, meta_head_key, pidx_delta_key, shard_key},
	takeover,
	types::{DBHead, DatabaseBranchId, encode_db_head},
};
use gas::prelude::Id;
use rivet_pools::NodeId;

fn head(head_txid: u64, db_size_pages: u32) -> DBHead {
	DBHead {
		head_txid,
		db_size_pages,
		post_apply_checksum: 0,
		branch_id: DatabaseBranchId::nil(),
		#[cfg(debug_assertions)]
		generation: 0,
	}
}

async fn seed(db: &universaldb::Database, writes: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
	db.run(move |tx| {
		let writes = writes.clone();
		async move {
			for (key, value) in writes {
				tx.informal().set(&key, &value);
			}
			Ok(())
		}
	})
	.await
}

#[tokio::test]
async fn legacy_database_scoped_rows_are_ignored() -> Result<()> {
	common::test_matrix("depot-takeover-legacy-ignored", |_tier, ctx| {
		Box::pin(async move {
			let db = ctx.udb.clone();
			let database_id = ctx.database_id.clone();
			seed(
				&db,
				vec![
					(meta_head_key(&database_id), encode_db_head(head(1, 1))?),
					(delta_chunk_key(&database_id, 99, 0), b"delta".to_vec()),
					(
						pidx_delta_key(&database_id, 99),
						99_u64.to_be_bytes().to_vec(),
					),
					(shard_key(&database_id, 99), b"shard".to_vec()),
				],
			)
			.await?;

			takeover::reconcile(&db, &database_id).await?;

			Ok(())
		})
	})
	.await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn db_new_does_not_wait_for_takeover_reconcile() -> Result<()> {
	let udb = common::test_db_arc("depot-takeover-nonblocking").await?;
	let bucket_id = Id::new_v1(1);
	let database_id = "depot-takeover-nonblocking-db".to_string();
	let node_id = NodeId::new();
	let (_guard, reached, release) = takeover::pause_reconcile_for_test(&database_id);

	let (constructor_done_tx, constructor_done_rx) = tokio::sync::oneshot::channel();
	let constructor = {
		let udb = udb.clone();
		let database_id = database_id.clone();
		tokio::spawn(async move {
			let _db = Db::new(udb, bucket_id, database_id, node_id);
			let _ = constructor_done_tx.send(());
		})
	};

	tokio::time::timeout(Duration::from_secs(2), reached.notified())
		.await
		.context("takeover reconcile did not start")?;

	let cpu_task = tokio::spawn(async {
		tokio::task::yield_now().await;
	});
	tokio::time::timeout(Duration::from_millis(100), cpu_task)
		.await
		.context("runtime worker did not make progress while reconcile was pending")??;

	tokio::time::timeout(Duration::from_millis(100), constructor_done_rx)
		.await
		.context("Db::new waited for takeover reconcile to finish")??;

	release.notify_waiters();
	constructor.await?;

	Ok(())
}
