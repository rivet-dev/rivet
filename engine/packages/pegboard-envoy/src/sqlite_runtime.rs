use std::sync::Arc;

use anyhow::{Context, Result};
use gas::prelude::StandaloneCtx;
use rivet_envoy_protocol as protocol;
use sqlite_storage::{
	keys,
	types::{FetchedPage, decode_db_head, decode_meta_compact},
};
use sqlite_storage_legacy::{engine::SqliteEngine, open::OpenResult};
use tokio::sync::OnceCell;
use universaldb::{Subspace, utils::IsolationLevel::Snapshot};

static SQLITE_ENGINE: OnceCell<Arc<SqliteEngine>> = OnceCell::const_new();

pub async fn shared_engine(ctx: &StandaloneCtx) -> Result<Arc<SqliteEngine>> {
	let db = (*ctx.udb()?).clone();
	let subspace = sqlite_subspace();

	SQLITE_ENGINE
		.get_or_try_init(|| async move {
			tracing::info!("initializing shared sqlite dispatch runtime");

			let (engine, _compaction_rx) = SqliteEngine::new(db, subspace.clone());
			let engine = Arc::new(engine);

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

pub fn protocol_sqlite_meta(meta: sqlite_storage_legacy::types::SqliteMeta) -> protocol::SqliteMeta {
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
	page: sqlite_storage_legacy::types::FetchedPage,
) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

pub async fn protocol_sqlite_pump_meta(
	db: &universaldb::Database,
	actor_id: &str,
) -> Result<protocol::SqliteMeta> {
	let actor_id = actor_id.to_string();
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let head_bytes = tx
				.informal()
				.get(&keys::meta_head_key(&actor_id), Snapshot)
				.await?
				.context("sqlite meta missing")?;
			let compact_bytes = tx
				.informal()
				.get(&keys::meta_compact_key(&actor_id), Snapshot)
				.await?;

			let head = decode_db_head(&head_bytes).context("decode sqlite pump head")?;
			let materialized_txid = compact_bytes
				.as_ref()
				.map(|bytes| decode_meta_compact(bytes.as_ref()))
				.transpose()
				.context("decode sqlite pump compact meta")?
				.map_or(0, |compact| compact.materialized_txid);

			Ok(protocol::SqliteMeta {
				#[cfg(debug_assertions)]
				generation: head.generation,
				#[cfg(not(debug_assertions))]
				generation: 0,
				head_txid: head.head_txid,
				materialized_txid,
				db_size_pages: head.db_size_pages,
				page_size: sqlite_storage::types::SQLITE_PAGE_SIZE,
				creation_ts_ms: 0,
				max_delta_bytes: u64::MAX,
			})
		}
	})
	.await
}

pub fn protocol_sqlite_pump_fetched_page(page: FetchedPage) -> protocol::SqliteFetchedPage {
	protocol::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}
