use std::sync::Arc;

use anyhow::Result;
use gas::prelude::StandaloneCtx;
use rivet_envoy_protocol as protocol;
use sqlite_storage::{compaction::CompactionCoordinator, engine::SqliteEngine, open::OpenResult};
use tokio::sync::OnceCell;
use universaldb::Subspace;

static SQLITE_ENGINE: OnceCell<Arc<SqliteEngine>> = OnceCell::const_new();

pub async fn shared_engine(ctx: &StandaloneCtx) -> Result<Arc<SqliteEngine>> {
	let db = (*ctx.udb()?).clone();
	let subspace = sqlite_subspace();

	SQLITE_ENGINE
		.get_or_try_init(|| async move {
			tracing::info!("initializing shared sqlite dispatch runtime");

			let (engine, compaction_rx) = SqliteEngine::new(db, subspace.clone());
			let engine = Arc::new(engine);
			tokio::spawn(CompactionCoordinator::run(
				compaction_rx,
				Arc::clone(&engine),
			));

			Ok(engine)
		})
		.await
		.cloned()
}

fn sqlite_subspace() -> Subspace {
	pegboard::keys::subspace().subspace(&("sqlite-storage",))
}

pub fn protocol_sqlite_startup_data(startup: OpenResult) -> protocol::SqliteStartupData {
	protocol::SqliteStartupData {
		generation: startup.generation,
		meta: protocol_sqlite_meta(startup.meta),
		preloaded_pages: startup
			.preloaded_pages
			.into_iter()
			.map(protocol_sqlite_fetched_page)
			.collect(),
	}
}

pub fn protocol_sqlite_meta(meta: sqlite_storage::types::SqliteMeta) -> protocol::SqliteMeta {
	protocol::SqliteMeta {
		generation: meta.generation,
		head_txid: meta.head_txid,
		materialized_txid: meta.materialized_txid,
		db_size_pages: meta.db_size_pages,
		page_size: meta.page_size,
		creation_ts_ms: meta.creation_ts_ms,
		max_delta_bytes: meta.max_delta_bytes,
	}
}

pub fn protocol_sqlite_fetched_page(
	page: sqlite_storage::types::FetchedPage,
) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}
