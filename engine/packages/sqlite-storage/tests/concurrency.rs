use std::sync::Arc;

use anyhow::{Context, Result};
use sqlite_storage::commit::CommitRequest;
use sqlite_storage::engine::SqliteEngine;
use sqlite_storage::open::OpenConfig;
use sqlite_storage::types::{DirtyPage, SQLITE_PAGE_SIZE};
use tempfile::Builder;
use tokio::sync::Barrier;
use tokio::task::JoinSet;
use tokio::task::yield_now;
use universaldb::Subspace;
use uuid::Uuid;

async fn setup_engine() -> Result<(SqliteEngine, tokio::sync::mpsc::UnboundedReceiver<String>)> {
	let path = Builder::new()
		.prefix("sqlite-storage-concurrency-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
	let db = universaldb::Database::new(Arc::new(driver));
	let subspace = Subspace::new(&("sqlite-storage-concurrency", Uuid::new_v4().to_string()));

	Ok(SqliteEngine::new(db, subspace))
}

fn dirty_pages(start_pgno: u32, count: u32, fill: u8) -> Vec<DirtyPage> {
	(0..count)
		.map(|offset| DirtyPage {
			pgno: start_pgno + offset,
			bytes: vec![fill; SQLITE_PAGE_SIZE as usize],
		})
		.collect()
}

fn page(fill: u8) -> Vec<u8> {
	vec![fill; SQLITE_PAGE_SIZE as usize]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_commits_to_different_actors_preserve_isolation() -> Result<()> {
	let (engine, _compaction_rx) = setup_engine().await?;
	let engine = Arc::new(engine);
	let mut actors = Vec::new();

	for idx in 0..10u8 {
		let actor_id = format!("actor-{idx}");
		let open = engine
			.open(&actor_id, OpenConfig::new(i64::from(idx) + 1))
			.await?;
		actors.push((actor_id, open.generation, open.meta.head_txid, idx));
	}

	let mut commits = JoinSet::new();
	for (actor_id, generation, head_txid, idx) in actors.clone() {
		let engine = Arc::clone(&engine);
		commits.spawn(async move {
			engine
				.commit(
					&actor_id,
					CommitRequest {
						generation,
						head_txid,
						db_size_pages: 1,
						dirty_pages: dirty_pages(1, 1, idx + 1),
						now_ms: i64::from(idx) + 100,
					},
				)
				.await
				.with_context(|| format!("commit for {actor_id}"))?;
			Ok::<_, anyhow::Error>((actor_id, generation, idx + 1))
		});
	}

	while let Some(result) = commits.join_next().await {
		let (actor_id, generation, fill) = result??;
		let pages = engine.get_pages(&actor_id, generation, vec![1]).await?;
		assert_eq!(pages[0].bytes, Some(page(fill)));
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn interleaved_commit_compaction_read_keeps_latest_page_visible() -> Result<()> {
	let (engine, _compaction_rx) = setup_engine().await?;
	let actor_id = "interleaved-actor";
	let open = engine.open(actor_id, OpenConfig::new(1)).await?;
	let first_commit = engine
		.commit(
			actor_id,
			CommitRequest {
				generation: open.generation,
				head_txid: open.meta.head_txid,
				db_size_pages: 70,
				dirty_pages: dirty_pages(1, 70, 0x11),
				now_ms: 2,
			},
		)
		.await?;
	assert!(engine.compact_shard(actor_id, 0).await?);

	let after_compaction = engine
		.get_pages(actor_id, open.generation, vec![1, 2])
		.await?;
	assert_eq!(after_compaction[0].bytes, Some(page(0x11)));
	assert_eq!(after_compaction[1].bytes, Some(page(0x11)));

	engine
		.commit(
			actor_id,
			CommitRequest {
				generation: open.generation,
				head_txid: first_commit.txid,
				db_size_pages: 70,
				dirty_pages: dirty_pages(1, 2, 0x44),
				now_ms: 3,
			},
		)
		.await?;

	let latest = engine
		.get_pages(actor_id, open.generation, vec![1, 2, 3])
		.await?;
	assert_eq!(latest[0].bytes, Some(page(0x44)));
	assert_eq!(latest[1].bytes, Some(page(0x44)));
	assert_eq!(latest[2].bytes, Some(page(0x11)));

	Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_reads_during_compaction_keep_returning_expected_pages() -> Result<()> {
	let (engine, _compaction_rx) = setup_engine().await?;
	let engine = Arc::new(engine);
	let actor_id = "read-compaction-actor".to_string();
	let open = engine.open(&actor_id, OpenConfig::new(10)).await?;
	let generation = open.generation;
	let mut head_txid = open.meta.head_txid;

	for (shard_idx, fill) in [(0_u32, 0x10_u8), (1, 0x20), (2, 0x30), (3, 0x40)] {
		let commit = engine
			.commit(
				&actor_id,
				CommitRequest {
					generation,
					head_txid,
					db_size_pages: 256,
					dirty_pages: dirty_pages(shard_idx * 64 + 1, 64, fill),
					now_ms: 20 + i64::from(shard_idx),
				},
			)
			.await?;
		head_txid = commit.txid;
	}

	let warmup = engine
		.get_pages(
			&actor_id,
			generation,
			vec![1, 2, 65, 66, 129, 130, 193, 194],
		)
		.await?;
	assert_eq!(warmup[0].bytes, Some(page(0x10)));
	assert_eq!(warmup[2].bytes, Some(page(0x20)));
	assert_eq!(warmup[4].bytes, Some(page(0x30)));
	assert_eq!(warmup[6].bytes, Some(page(0x40)));

	let barrier = Arc::new(Barrier::new(6));
	let mut tasks = JoinSet::new();

	{
		let engine = Arc::clone(&engine);
		let barrier = Arc::clone(&barrier);
		let actor_id = actor_id.clone();
		tasks.spawn(async move {
			barrier.wait().await;
			engine.compact_default_batch(&actor_id).await?;
			Ok::<_, anyhow::Error>(())
		});
	}

	for _ in 0..4 {
		let engine = Arc::clone(&engine);
		let barrier = Arc::clone(&barrier);
		let actor_id = actor_id.clone();
		tasks.spawn(async move {
			barrier.wait().await;
			for _ in 0..20 {
				let pages = engine
					.get_pages(
						&actor_id,
						generation,
						vec![1, 2, 65, 66, 129, 130, 193, 194],
					)
					.await?;
				assert_eq!(pages[0].bytes, Some(page(0x10)));
				assert_eq!(pages[1].bytes, Some(page(0x10)));
				assert_eq!(pages[2].bytes, Some(page(0x20)));
				assert_eq!(pages[3].bytes, Some(page(0x20)));
				assert_eq!(pages[4].bytes, Some(page(0x30)));
				assert_eq!(pages[5].bytes, Some(page(0x30)));
				assert_eq!(pages[6].bytes, Some(page(0x40)));
				assert_eq!(pages[7].bytes, Some(page(0x40)));
				yield_now().await;
			}
			Ok::<_, anyhow::Error>(())
		});
	}

	barrier.wait().await;
	while let Some(result) = tasks.join_next().await {
		result??;
	}

	let final_pages = engine
		.get_pages(&actor_id, generation, vec![1, 65, 129, 193])
		.await?;
	assert_eq!(final_pages[0].bytes, Some(page(0x10)));
	assert_eq!(final_pages[1].bytes, Some(page(0x20)));
	assert_eq!(final_pages[2].bytes, Some(page(0x30)));
	assert_eq!(final_pages[3].bytes, Some(page(0x40)));

	Ok(())
}

#[tokio::test]
async fn second_open_for_same_actor_takes_over_and_fences_old_generation() -> Result<()> {
	let (engine, _compaction_rx) = setup_engine().await?;
	let actor_id = "double-open-actor";

	let first = engine.open(actor_id, OpenConfig::new(1)).await?;
	let second = engine.open(actor_id, OpenConfig::new(2)).await?;

	assert!(
		second.generation > first.generation,
		"takeover generation {} must fence old generation {}",
		second.generation,
		first.generation
	);

	let err = engine
		.commit(
			actor_id,
			CommitRequest {
				generation: first.generation,
				head_txid: first.meta.head_txid,
				db_size_pages: 1,
				dirty_pages: dirty_pages(1, 1, 0x55),
				now_ms: 3,
			},
		)
		.await
		.expect_err("old generation must be fenced after takeover");
	assert!(
		err.to_string().contains("did not match open generation"),
		"unexpected stale-generation error: {err}"
	);

	engine
		.commit(
			actor_id,
			CommitRequest {
				generation: second.generation,
				head_txid: second.meta.head_txid,
				db_size_pages: 1,
				dirty_pages: dirty_pages(1, 1, 0x66),
				now_ms: 4,
			},
		)
		.await?;
	engine.close(actor_id, second.generation).await?;

	Ok(())
}
