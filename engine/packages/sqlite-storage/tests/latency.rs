use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::Result;
use sqlite_storage::commit::CommitRequest;
use sqlite_storage::engine::SqliteEngine;
use sqlite_storage::open::OpenConfig;
use sqlite_storage::types::{DirtyPage, SQLITE_PAGE_SIZE};
use tempfile::Builder;
use tokio::time::sleep;
use universaldb::Subspace;
use uuid::Uuid;

async fn setup_engine() -> Result<(SqliteEngine, tokio::sync::mpsc::UnboundedReceiver<String>)> {
	let path = Builder::new()
		.prefix("sqlite-storage-latency-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
	let db = universaldb::Database::new(Arc::new(driver));
	let subspace = Subspace::new(&("sqlite-storage-latency", Uuid::new_v4().to_string()));

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

fn assert_single_rtt(label: &str, elapsed: Duration) {
	assert!(
		elapsed >= Duration::from_millis(18),
		"{label} finished too quickly for 20 ms injected latency: {elapsed:?}",
	);
	assert!(
		elapsed < Duration::from_millis(45),
		"{label} took longer than a single RTT under 20 ms injected latency: {elapsed:?}",
	);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn latency_paths_use_single_rtt_under_simulated_udb_latency() -> Result<()> {
	unsafe {
		std::env::set_var("UDB_SIMULATED_LATENCY_MS", "20");
	}

	{
		let (engine, _compaction_rx) = setup_engine().await?;
		let open = engine
			.open("latency-small-commit", OpenConfig::new(1))
			.await?;
		engine.op_counter.store(0, Ordering::SeqCst);

		let started_at = Instant::now();
		engine
			.commit(
				"latency-small-commit",
				CommitRequest {
					generation: open.generation,
					head_txid: open.meta.head_txid,
					db_size_pages: 4,
					dirty_pages: dirty_pages(1, 4, 0x11),
					now_ms: 2,
				},
			)
			.await?;
		let elapsed = started_at.elapsed();

		assert_eq!(engine.op_counter.load(Ordering::SeqCst), 1);
		assert_single_rtt("small commit", elapsed);
	}

	{
		let (engine, _compaction_rx) = setup_engine().await?;
		let open = engine.open("latency-get-pages", OpenConfig::new(3)).await?;
		let commit = engine
			.commit(
				"latency-get-pages",
				CommitRequest {
					generation: open.generation,
					head_txid: open.meta.head_txid,
					db_size_pages: 10,
					dirty_pages: dirty_pages(1, 10, 0x22),
					now_ms: 4,
				},
			)
			.await?;
		assert_eq!(commit.txid, 1);
		engine.op_counter.store(0, Ordering::SeqCst);

		let started_at = Instant::now();
		let pages = engine
			.get_pages("latency-get-pages", open.generation, (1..=10).collect())
			.await?;
		let elapsed = started_at.elapsed();

		assert!(pages.iter().all(|page| page.bytes.is_some()));
		assert_eq!(engine.op_counter.load(Ordering::SeqCst), 1);
		assert_single_rtt("get_pages", elapsed);
	}

	{
		let (engine, mut compaction_rx) = setup_engine().await?;
		let open = engine
			.open("latency-compaction", OpenConfig::new(5))
			.await?;
		let compaction_task = tokio::spawn(async move {
			let actor_id = compaction_rx
				.recv()
				.await
				.expect("commit should enqueue compaction work");
			sleep(Duration::from_millis(200)).await;
			actor_id
		});
		engine.op_counter.store(0, Ordering::SeqCst);

		let started_at = Instant::now();
		engine
			.commit(
				"latency-compaction",
				CommitRequest {
					generation: open.generation,
					head_txid: open.meta.head_txid,
					db_size_pages: 4,
					dirty_pages: dirty_pages(1, 4, 0x33),
					now_ms: 6,
				},
			)
			.await?;
		let elapsed = started_at.elapsed();

		assert_eq!(engine.op_counter.load(Ordering::SeqCst), 1);
		assert_single_rtt("commit during compaction queueing", elapsed);
		assert_eq!(compaction_task.await?, "latency-compaction".to_string());
	}

	Ok(())
}
