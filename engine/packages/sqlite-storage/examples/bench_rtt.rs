//! RTT benchmark for sqlite-storage operations.
//!
//! Measures wall-clock time and UDB op counts for commit and get_pages under
//! various page counts. Run with and without UDB_SIMULATED_LATENCY_MS=20 to
//! project remote-database round-trip costs.
//!
//! Usage:
//!   cargo run -p sqlite-storage --example bench_rtt
//!   UDB_SIMULATED_LATENCY_MS=20 cargo run -p sqlite-storage --example bench_rtt

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use anyhow::{Context, Result};
use tempfile::Builder;
use uuid::Uuid;

use sqlite_storage::commit::{
	CommitFinalizeRequest, CommitRequest, CommitStageBeginRequest, CommitStageRequest,
};
use sqlite_storage::engine::SqliteEngine;
use sqlite_storage::ltx::{LtxHeader, encode_ltx_v3};
use sqlite_storage::takeover::TakeoverConfig;
use sqlite_storage::types::{DirtyPage, SQLITE_PAGE_SIZE};
use universaldb::Subspace;

async fn setup() -> Result<(SqliteEngine, tokio::sync::mpsc::UnboundedReceiver<String>)> {
	let path = Builder::new().prefix("bench-rtt-").tempdir()?.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;
	let db = Arc::new(universaldb::Database::new(Arc::new(driver)));
	let subspace = Subspace::new(&("bench-rtt", Uuid::new_v4().to_string()));

	Ok(SqliteEngine::new(db, subspace))
}

fn make_pages(count: u32, fill: u8) -> Vec<DirtyPage> {
	(1..=count)
		.map(|pgno| DirtyPage {
			pgno,
			bytes: vec![fill; SQLITE_PAGE_SIZE as usize],
		})
		.collect()
}

fn clear_ops(engine: &SqliteEngine) {
	engine.op_counter.store(0, Ordering::SeqCst);
}

fn read_ops(engine: &SqliteEngine) -> usize {
	engine.op_counter.load(Ordering::SeqCst)
}

struct BenchResult {
	label: &'static str,
	actor_rts: usize,
	udb_txs: usize,
	wall_ms: f64,
}

impl BenchResult {
	fn projected_ms(&self, rtt_ms: f64) -> f64 {
		self.actor_rts as f64 * rtt_ms
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let simulated_ms: u64 = std::env::var("UDB_SIMULATED_LATENCY_MS")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(0);
	let projected_rtt_ms = 20.0;

	println!("=== sqlite-storage RTT benchmark ===");
	println!(
		"UDB_SIMULATED_LATENCY_MS = {} ({})",
		simulated_ms,
		if simulated_ms > 0 {
			"latency injection active"
		} else {
			"local only"
		}
	);
	println!();
	println!(
		"actor_rts uses direct-engine calls and is hardcoded to 1 per scenario until end-to-end VFS+envoy measurement exists."
	);
	println!();

	let mut results = Vec::new();

	{
		let (engine, _rx) = setup().await?;
		let takeover = engine
			.takeover("bench-small", TakeoverConfig::new(1))
			.await
			.context("takeover for small commit")?;
		clear_ops(&engine);

		let start = Instant::now();
		engine
			.commit(
				"bench-small",
				CommitRequest {
					generation: takeover.generation,
					head_txid: takeover.meta.head_txid,
					db_size_pages: 10,
					dirty_pages: make_pages(10, 0xAA),
					now_ms: 100,
				},
			)
			.await
			.context("small commit")?;
		let elapsed = start.elapsed();

		results.push(BenchResult {
			label: "commit 10 pages (small)",
			actor_rts: 1,
			udb_txs: read_ops(&engine),
			wall_ms: elapsed.as_secs_f64() * 1000.0,
		});
	}

	{
		let (engine, _rx) = setup().await?;
		let takeover = engine
			.takeover("bench-medium", TakeoverConfig::new(2))
			.await
			.context("takeover for medium commit")?;
		clear_ops(&engine);

		let start = Instant::now();
		engine
			.commit(
				"bench-medium",
				CommitRequest {
					generation: takeover.generation,
					head_txid: takeover.meta.head_txid,
					db_size_pages: 256,
					dirty_pages: make_pages(256, 0xBB),
					now_ms: 200,
				},
			)
			.await
			.context("medium commit")?;
		let elapsed = start.elapsed();

		results.push(BenchResult {
			label: "commit 256 pages / 1 MiB (medium)",
			actor_rts: 1,
			udb_txs: read_ops(&engine),
			wall_ms: elapsed.as_secs_f64() * 1000.0,
		});
	}

	{
		let (engine, _rx) = setup().await?;
		let takeover = engine
			.takeover("bench-large", TakeoverConfig::new(3))
			.await
			.context("takeover for large commit")?;
		clear_ops(&engine);

		let total_pages = 2560_u32;
		let stage = engine
			.commit_stage_begin(
				"bench-large",
				CommitStageBeginRequest {
					generation: takeover.generation,
				},
			)
			.await
			.context("large commit stage begin")?;
		let encoded = encode_ltx_v3(
			LtxHeader::delta(stage.txid, total_pages, 300),
			&make_pages(total_pages, 0xCC),
		)?;
		let chunk_bytes = 128_usize * SQLITE_PAGE_SIZE as usize;
		let chunks = encoded.chunks(chunk_bytes).count();
		let start = Instant::now();
		for (chunk_idx, chunk) in encoded.chunks(chunk_bytes).enumerate() {
			let is_last = chunk_idx == chunks - 1;
			engine
				.commit_stage(
					"bench-large",
					CommitStageRequest {
						generation: takeover.generation,
						txid: stage.txid,
						chunk_idx: chunk_idx as u32,
						bytes: chunk.to_vec(),
						is_last,
					},
				)
				.await
				.with_context(|| format!("large commit stage chunk {chunk_idx}"))?;
		}
		engine
			.commit_finalize(
				"bench-large",
				CommitFinalizeRequest {
					generation: takeover.generation,
					expected_head_txid: takeover.meta.head_txid,
					txid: stage.txid,
					new_db_size_pages: total_pages,
					now_ms: 300,
					origin_override: None,
				},
			)
			.await
			.context("large commit finalize")?;
		let elapsed = start.elapsed();

		results.push(BenchResult {
			label: "commit 2560 pages / 10 MiB (large, staged)",
			actor_rts: 1,
			udb_txs: read_ops(&engine),
			wall_ms: elapsed.as_secs_f64() * 1000.0,
		});
	}

	{
		let (engine, _rx) = setup().await?;
		let takeover = engine
			.takeover("bench-read", TakeoverConfig::new(4))
			.await
			.context("takeover for read bench")?;

		engine
			.commit(
				"bench-read",
				CommitRequest {
					generation: takeover.generation,
					head_txid: takeover.meta.head_txid,
					db_size_pages: 50,
					dirty_pages: make_pages(50, 0xDD),
					now_ms: 400,
				},
			)
			.await
			.context("seed pages for read bench")?;
		clear_ops(&engine);

		let read_pgnos = vec![3, 7, 11, 15, 19, 23, 27, 31, 35, 42];
		let start = Instant::now();
		let _pages = engine
			.get_pages("bench-read", takeover.generation, read_pgnos)
			.await
			.context("get_pages bench")?;
		let elapsed = start.elapsed();

		results.push(BenchResult {
			label: "get_pages 10 random pages",
			actor_rts: 1,
			udb_txs: read_ops(&engine),
			wall_ms: elapsed.as_secs_f64() * 1000.0,
		});
	}

	for result in &results {
		println!(
			"{} | actor_rts: {} | udb_txs: {} | wall_ms: {:.2} | projected_ms: {:.1}",
			result.label,
			result.actor_rts,
			result.udb_txs,
			result.wall_ms,
			result.projected_ms(projected_rtt_ms)
		);
	}

	println!();
	if simulated_ms > 0 {
		println!(
			"With {}ms simulated latency, the wall-clock times above include the injected UDB delay.",
			simulated_ms
		);
	} else {
		println!("Run with UDB_SIMULATED_LATENCY_MS=20 to simulate remote database latency.");
	}

	Ok(())
}
