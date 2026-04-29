use std::sync::Arc;

use anyhow::Result;
use gas::prelude::StandaloneCtx;
use rivet_envoy_protocol as protocol;
use sqlite_storage::{engine::SqliteEngine, open::OpenResult};

pub async fn shared_engine(ctx: &StandaloneCtx) -> Result<Arc<SqliteEngine>> {
	pegboard::actor_sqlite::shared_engine(ctx).await
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

pub fn storage_preload_hints(
	hints: protocol::SqlitePreloadHints,
) -> sqlite_storage::types::PreloadHints {
	sqlite_storage::types::PreloadHints {
		pgnos: hints.pgnos,
		ranges: hints
			.ranges
			.into_iter()
			.map(|range| sqlite_storage::types::PreloadHintRange {
				start_pgno: range.start_pgno,
				page_count: range.page_count,
			})
			.collect(),
	}
}
