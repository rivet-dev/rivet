use std::sync::Arc;

use anyhow::Result;
use gas::prelude::{Id, StandaloneCtx, util::timestamp};
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION};
use sqlite_storage::{
	compaction::CompactionCoordinator, engine::SqliteEngine, takeover::TakeoverConfig,
	types::SQLITE_VFS_V2_SCHEMA_VERSION,
};
use tokio::sync::OnceCell;
use universaldb::Subspace;

static SQLITE_ENGINE: OnceCell<Arc<SqliteEngine>> = OnceCell::const_new();

pub async fn shared_engine(ctx: &StandaloneCtx) -> Result<Arc<SqliteEngine>> {
	let db = Arc::new((*ctx.udb()?).clone());
	let subspace = sqlite_subspace();

	SQLITE_ENGINE
		.get_or_try_init(|| async move {
			tracing::info!("initializing shared sqlite dispatch runtime");

			let (engine, compaction_rx) = SqliteEngine::new(Arc::clone(&db), subspace.clone());
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

pub async fn populate_start_command(
	ctx: &StandaloneCtx,
	sqlite_engine: &SqliteEngine,
	protocol_version: u16,
	namespace_id: Id,
	actor_id: Id,
	start: &mut protocol::CommandStartActor,
) -> Result<()> {
	if start.preloaded_kv.is_none() {
		let db = ctx.udb()?;
		start.preloaded_kv = pegboard::actor_kv::preload::fetch_preloaded_kv(
			&db,
			ctx.config().pegboard(),
			actor_id,
			namespace_id,
			&start.config.name,
		)
		.await?;
	}

	start.sqlite_schema_version =
		if pegboard::actor_kv::sqlite_v1_data_exists(&*ctx.udb()?, actor_id).await? {
			pegboard::workflows::actor2::SQLITE_SCHEMA_VERSION_V1
		} else {
			SQLITE_VFS_V2_SCHEMA_VERSION
		};
	start.sqlite_startup_data = maybe_load_sqlite_startup_data(
		sqlite_engine,
		protocol_version,
		actor_id,
		start.sqlite_schema_version,
	)
	.await?;

	Ok(())
}

pub async fn maybe_load_sqlite_startup_data(
	sqlite_engine: &SqliteEngine,
	protocol_version: u16,
	actor_id: Id,
	sqlite_schema_version: u32,
) -> Result<Option<protocol::SqliteStartupData>> {
	if sqlite_schema_version != SQLITE_VFS_V2_SCHEMA_VERSION || protocol_version < PROTOCOL_VERSION
	{
		return Ok(None);
	}

	let actor_id = actor_id.to_string();
	let startup = sqlite_engine
		.takeover(&actor_id, TakeoverConfig::new(timestamp::now()))
		.await?;

	Ok(Some(protocol::SqliteStartupData {
		generation: startup.generation,
		meta: protocol_sqlite_meta(startup.meta),
		preloaded_pages: startup
			.preloaded_pages
			.into_iter()
			.map(protocol_sqlite_fetched_page)
			.collect(),
	}))
}

pub fn protocol_sqlite_meta(meta: sqlite_storage::types::SqliteMeta) -> protocol::SqliteMeta {
	protocol::SqliteMeta {
		schema_version: meta.schema_version,
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
