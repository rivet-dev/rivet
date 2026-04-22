use std::sync::{Arc, OnceLock};
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use gas::prelude::{Id, StandaloneCtx, util::timestamp};
use pegboard::actor_kv::Recipient;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION};
use rusqlite::Connection;
use scc::{HashMap, hash_map::Entry};
use sqlite_storage::{
	commit::{CommitFinalizeRequest, CommitStageBeginRequest, CommitStageRequest},
	compaction::CompactionCoordinator,
	engine::SqliteEngine,
	ltx::{LtxHeader, encode_ltx_v3},
	takeover::TakeoverConfig,
	types::{DirtyPage, SQLITE_PAGE_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin},
};
use tempfile::tempdir;
use tokio::sync::{Mutex, OnceCell};

use crate::metrics;

static SQLITE_ENGINE: OnceCell<Arc<SqliteEngine>> = OnceCell::const_new();
static SQLITE_MIGRATION_LOCKS: OnceLock<HashMap<String, Arc<Mutex<()>>>> = OnceLock::new();

const SQLITE_V1_PREFIX: u8 = 0x08;
const SQLITE_V1_SCHEMA_VERSION: u8 = 0x01;
const SQLITE_V1_META_PREFIX: u8 = 0x00;
const SQLITE_V1_CHUNK_PREFIX: u8 = 0x01;
const SQLITE_V1_META_VERSION: u16 = 1;
const SQLITE_V1_META_LEN: usize = 10;
const SQLITE_V1_CHUNK_SIZE: usize = 4096;
const SQLITE_V1_MAX_MIGRATION_BYTES: u64 = 128 * 1024 * 1024;
const SQLITE_V1_MIGRATION_LEASE_MS: i64 = 5 * 60 * 1000;
const FILE_TAG_MAIN: u8 = 0x00;
const FILE_TAG_JOURNAL: u8 = 0x01;
const FILE_TAG_WAL: u8 = 0x02;
const FILE_TAG_SHM: u8 = 0x03;
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

pub async fn shared_engine(ctx: &StandaloneCtx) -> Result<Arc<SqliteEngine>> {
	let db = Arc::new((*ctx.udb()?).clone());
	let subspace = pegboard::actor_sqlite_v2::sqlite_subspace();

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

fn migration_locks() -> &'static HashMap<String, Arc<Mutex<()>>> {
	SQLITE_MIGRATION_LOCKS.get_or_init(HashMap::default)
}

async fn actor_migration_lock(actor_id: &str) -> Arc<Mutex<()>> {
	let actor_id = actor_id.to_string();
	match migration_locks().entry_async(actor_id).await {
		Entry::Occupied(entry) => Arc::clone(entry.get()),
		Entry::Vacant(entry) => Arc::clone(entry.insert_entry(Arc::new(Mutex::new(()))).get()),
	}
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

	let db = ctx.udb()?;
	let recipient = Recipient {
		actor_id,
		namespace_id,
		name: start.config.name.clone(),
	};
	if protocol_version >= PROTOCOL_VERSION {
		maybe_migrate_v1_to_v2(&db, sqlite_engine, &recipient).await?;
	}

	let actor_id_str = actor_id.to_string();
	let has_v2_meta = sqlite_engine.try_load_meta(&actor_id_str).await?.is_some();
	start.sqlite_schema_version = if has_v2_meta {
		SQLITE_VFS_V2_SCHEMA_VERSION
	} else if pegboard::actor_kv::sqlite_v1_data_exists(&db, actor_id).await? {
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

async fn maybe_migrate_v1_to_v2(
	db: &universaldb::Database,
	sqlite_engine: &SqliteEngine,
	recipient: &Recipient,
) -> Result<bool> {
	if !pegboard::actor_kv::sqlite_v1_data_exists(db, recipient.actor_id).await? {
		return Ok(false);
	}

	let actor_id = recipient.actor_id.to_string();
	let migration_lock = actor_migration_lock(&actor_id).await;
	let _guard = migration_lock.lock().await;

	if !pegboard::actor_kv::sqlite_v1_data_exists(db, recipient.actor_id).await? {
		return Ok(false);
	}

	if let Some(head) = sqlite_engine.try_load_head(&actor_id).await? {
		match head.origin {
			SqliteOrigin::Native | SqliteOrigin::MigratedFromV1 => return Ok(false),
			SqliteOrigin::MigratingFromV1 => {
				let migration_started_at = head.creation_ts_ms;
				let lease_expires_at =
					migration_started_at.saturating_add(SQLITE_V1_MIGRATION_LEASE_MS);
				let stage_in_progress = head.next_txid > head.head_txid.saturating_add(1);
				ensure!(
					!stage_in_progress || lease_expires_at <= timestamp::now(),
					"sqlite v1 migration for actor {actor_id} is already in progress"
				);
			}
		}
	}

	metrics::SQLITE_MIGRATION_ATTEMPTS_TOTAL.inc();
	let start = Instant::now();

	let snapshot = read_v1_snapshot(db, recipient)
		.await
		.map_err(|err| migration_error(&actor_id, "read_v1", err))?;
	let recovered = recover_v1_snapshot(&actor_id, snapshot)
		.map_err(|err| migration_error(&actor_id, "validate", err))?;
	metrics::SQLITE_MIGRATION_PAGES.observe(recovered.total_pages as f64);
	tracing::info!(
		actor_id = %actor_id,
		pages = recovered.total_pages,
		size_bytes = recovered.bytes.len(),
		has_journal = recovered.had_journal,
		"starting v1→v2 migration"
	);

	let prepared = sqlite_engine
		.prepare_v1_migration(&actor_id, timestamp::now())
		.await
		.map_err(|err| migration_error(&actor_id, "takeover", err))?;
	let stage_begin = sqlite_engine
		.commit_stage_begin(
			&actor_id,
			CommitStageBeginRequest {
				generation: prepared.meta.generation,
			},
		)
		.await
		.map_err(|err| migration_error(&actor_id, "stage", err))?;
	let dirty_pages = recovered
		.bytes
		.chunks(SQLITE_PAGE_SIZE as usize)
		.enumerate()
		.map(|(idx, bytes)| DirtyPage {
			pgno: idx as u32 + 1,
			bytes: bytes.to_vec(),
		})
		.collect::<Vec<_>>();
	let encoded_delta = encode_ltx_v3(
		LtxHeader::delta(stage_begin.txid, recovered.total_pages, timestamp::now()),
		&dirty_pages,
	)
	.map_err(|err| migration_error(&actor_id, "stage", err.into()))?;
	let staged_chunks = split_bytes(
		&encoded_delta,
		prepared
			.meta
			.max_delta_bytes
			.try_into()
			.context("sqlite max_delta_bytes exceeded usize")
			.map_err(|err| migration_error(&actor_id, "stage", err))?,
	);
	for (chunk_idx, chunk) in staged_chunks.iter().enumerate() {
		sqlite_engine
			.commit_stage(
				&actor_id,
				CommitStageRequest {
					generation: prepared.meta.generation,
					txid: stage_begin.txid,
					chunk_idx: chunk_idx as u32,
					bytes: chunk.clone(),
					is_last: chunk_idx + 1 == staged_chunks.len(),
				},
			)
			.await
			.map_err(|err| migration_error(&actor_id, "stage", err))?;
	}
	sqlite_engine
		.commit_finalize(
			&actor_id,
			CommitFinalizeRequest {
				generation: prepared.meta.generation,
				expected_head_txid: prepared.meta.head_txid,
				txid: stage_begin.txid,
				new_db_size_pages: recovered.total_pages,
				now_ms: timestamp::now(),
				origin_override: Some(SqliteOrigin::MigratedFromV1),
			},
		)
		.await
		.map_err(|err| migration_error(&actor_id, "finalize", err))?;

	metrics::SQLITE_MIGRATION_SUCCESSES_TOTAL.inc();
	metrics::SQLITE_MIGRATION_DURATION.observe(start.elapsed().as_secs_f64());
	tracing::info!(
		actor_id = %actor_id,
		pages = recovered.total_pages,
		duration_ms = start.elapsed().as_millis(),
		"v1→v2 migration complete"
	);

	Ok(true)
}

fn migration_error(actor_id: &str, phase: &'static str, err: anyhow::Error) -> anyhow::Error {
	metrics::SQLITE_MIGRATION_FAILURES_TOTAL
		.with_label_values(&[phase])
		.inc();
	tracing::error!(actor_id = %actor_id, phase, ?err, "v1→v2 migration failed");
	err
}

async fn read_v1_snapshot(
	db: &universaldb::Database,
	recipient: &Recipient,
) -> Result<RecoveredV1Snapshot> {
	ensure!(
		!v1_file_exists(db, recipient, FILE_TAG_WAL).await?,
		"unexpected sqlite v1 WAL sidecar present"
	);
	ensure!(
		!v1_file_exists(db, recipient, FILE_TAG_SHM).await?,
		"unexpected sqlite v1 SHM sidecar present"
	);

	let main = read_v1_file(db, recipient, FILE_TAG_MAIN)
		.await?
		.context("sqlite v1 main file missing metadata")?;
	let journal = read_v1_file(db, recipient, FILE_TAG_JOURNAL).await?;
	let had_journal = journal.is_some();

	Ok(RecoveredV1Snapshot {
		main,
		journal,
		had_journal,
	})
}

async fn v1_file_exists(
	db: &universaldb::Database,
	recipient: &Recipient,
	file_tag: u8,
) -> Result<bool> {
	let (keys, _, _) = pegboard::actor_kv::list(
		db,
		recipient,
		protocol::KvListQuery::KvListPrefixQuery(protocol::KvListPrefixQuery {
			key: v1_chunk_prefix(file_tag).to_vec(),
		}),
		false,
		Some(1),
	)
	.await?;

	Ok(!keys.is_empty())
}

async fn read_v1_file(
	db: &universaldb::Database,
	recipient: &Recipient,
	file_tag: u8,
) -> Result<Option<V1File>> {
	let meta_key = v1_meta_key(file_tag).to_vec();
	let (meta_keys, meta_values, _) =
		pegboard::actor_kv::get(db, recipient, vec![meta_key.clone()]).await?;

	if meta_keys.is_empty() && !v1_file_exists(db, recipient, file_tag).await? {
		return Ok(None);
	}
	ensure!(
		!meta_keys.is_empty(),
		"sqlite v1 file tag {file_tag} has chunks but no metadata"
	);
	ensure!(
		meta_keys.len() == 1 && meta_keys[0] == meta_key,
		"unexpected sqlite v1 metadata layout for file tag {file_tag}"
	);

	let size_bytes = decode_v1_meta(&meta_values[0])
		.with_context(|| format!("decode sqlite v1 metadata for file tag {file_tag}"))?;
	ensure!(
		size_bytes <= SQLITE_V1_MAX_MIGRATION_BYTES,
		"sqlite v1 file tag {file_tag} exceeded migration limit of {} bytes",
		SQLITE_V1_MAX_MIGRATION_BYTES
	);
	let expected_chunks = size_bytes.div_ceil(SQLITE_V1_CHUNK_SIZE as u64);
	let chunk_limit = usize::try_from(expected_chunks)
		.context("sqlite v1 expected chunk count exceeded usize")?
		.checked_add(1)
		.context("sqlite v1 chunk limit overflow")?
		.max(1);
	let (chunk_keys, chunk_values, _) = pegboard::actor_kv::list(
		db,
		recipient,
		protocol::KvListQuery::KvListPrefixQuery(protocol::KvListPrefixQuery {
			key: v1_chunk_prefix(file_tag).to_vec(),
		}),
		false,
		Some(chunk_limit),
	)
	.await?;
	let mut chunks = chunk_keys
		.into_iter()
		.zip(chunk_values.into_iter())
		.map(|(key, value)| {
			let chunk_idx = decode_v1_chunk_index(file_tag, &key)?;
			Ok((chunk_idx, value))
		})
		.collect::<Result<Vec<_>>>()?;
	chunks.sort_by_key(|(chunk_idx, _)| *chunk_idx);

	let bytes = rebuild_v1_file(
		size_bytes,
		expected_chunks
			.try_into()
			.context("sqlite v1 expected chunk count exceeded usize")?,
		&chunks,
	)
	.with_context(|| format!("rebuild sqlite v1 file tag {file_tag}"))?;

	Ok(Some(V1File { size_bytes, bytes }))
}

fn recover_v1_snapshot(actor_id: &str, snapshot: RecoveredV1Snapshot) -> Result<RecoveredDb> {
	if snapshot.main.size_bytes == 0 {
		return Ok(RecoveredDb {
			bytes: Vec::new(),
			total_pages: 0,
			had_journal: snapshot.had_journal,
		});
	}

	let tmp = tempdir().context("create sqlite v1 migration tempdir")?;
	let db_path = tmp.path().join("migration.db");
	std::fs::write(&db_path, &snapshot.main.bytes)
		.with_context(|| format!("write sqlite v1 main temp file for actor {actor_id}"))?;
	if let Some(journal) = snapshot.journal {
		std::fs::write(tmp.path().join("migration.db-journal"), &journal.bytes)
			.with_context(|| format!("write sqlite v1 journal temp file for actor {actor_id}"))?;
	}

	let conn = Connection::open(&db_path)
		.with_context(|| format!("open sqlite v1 temp db for actor {actor_id}"))?;
	conn.pragma_update(None, "journal_mode", "DELETE")
		.context("set sqlite journal_mode during v1 recovery")?;
	let integrity: String = conn
		.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
		.context("run sqlite quick_check during v1 recovery")?;
	ensure!(
		integrity == "ok",
		"sqlite integrity check failed after v1 recovery: {integrity}"
	);
	drop(conn);

	let recovered = std::fs::read(&db_path)
		.with_context(|| format!("read recovered sqlite db for actor {actor_id}"))?;
	ensure!(
		recovered.len() >= SQLITE_MAGIC.len() + 2,
		"sqlite v1 database too small after recovery"
	);
	ensure!(
		&recovered[..SQLITE_MAGIC.len()] == SQLITE_MAGIC,
		"sqlite magic bytes mismatch after v1 recovery"
	);
	let raw_page_size = u16::from_be_bytes([recovered[16], recovered[17]]);
	let page_size = if raw_page_size == 1 {
		65_536_u32
	} else {
		u32::from(raw_page_size)
	};
	ensure!(
		(512..=65_536).contains(&page_size),
		"sqlite page size {page_size} is outside the supported range"
	);
	ensure!(
		page_size == SQLITE_PAGE_SIZE,
		"sqlite page size {page_size} is not supported by sqlite v2"
	);
	ensure!(
		recovered.len() % page_size as usize == 0,
		"sqlite v1 database size {} is not page aligned to {}",
		recovered.len(),
		page_size
	);

	Ok(RecoveredDb {
		total_pages: (recovered.len() / page_size as usize) as u32,
		bytes: recovered,
		had_journal: snapshot.had_journal,
	})
}

fn decode_v1_meta(bytes: &[u8]) -> Result<u64> {
	ensure!(
		bytes.len() == SQLITE_V1_META_LEN,
		"sqlite v1 metadata had invalid length {}",
		bytes.len()
	);
	let version = u16::from_le_bytes(
		bytes[..2]
			.try_into()
			.expect("sqlite v1 metadata version bytes should exist"),
	);
	ensure!(
		version == SQLITE_V1_META_VERSION,
		"unsupported sqlite v1 metadata version {version}"
	);
	Ok(u64::from_le_bytes(
		bytes[2..10]
			.try_into()
			.expect("sqlite v1 metadata size bytes should exist"),
	))
}

fn rebuild_v1_file(
	size_bytes: u64,
	expected_chunks: usize,
	chunks: &[(u32, Vec<u8>)],
) -> Result<Vec<u8>> {
	let size_bytes: usize = size_bytes
		.try_into()
		.context("sqlite v1 file exceeded usize")?;
	ensure!(
		chunks.len() == expected_chunks,
		"sqlite v1 file expected {expected_chunks} chunks for size {size_bytes}, found {}",
		chunks.len()
	);
	let mut bytes = vec![0; size_bytes];

	for (expected_chunk_idx, (chunk_idx, chunk)) in chunks.iter().enumerate() {
		ensure!(
			*chunk_idx == expected_chunk_idx as u32,
			"sqlite v1 file missing or duplicated chunk at index {expected_chunk_idx}"
		);
		ensure!(
			chunk.len() <= SQLITE_V1_CHUNK_SIZE,
			"sqlite v1 chunk {chunk_idx} exceeded {} bytes",
			SQLITE_V1_CHUNK_SIZE
		);
		let start = (*chunk_idx as usize)
			.checked_mul(SQLITE_V1_CHUNK_SIZE)
			.context("sqlite v1 chunk offset overflow")?;
		let end = start
			.checked_add(chunk.len())
			.context("sqlite v1 chunk end overflow")?;
		ensure!(
			end <= bytes.len(),
			"sqlite v1 chunk {chunk_idx} overflowed file size {}",
			bytes.len()
		);
		bytes[start..end].copy_from_slice(chunk);
	}

	Ok(bytes)
}

fn decode_v1_chunk_index(file_tag: u8, key: &[u8]) -> Result<u32> {
	let prefix = v1_chunk_prefix(file_tag);
	ensure!(
		key.starts_with(&prefix),
		"sqlite v1 chunk key for file tag {file_tag} had the wrong prefix"
	);
	ensure!(
		key.len() == prefix.len() + 4,
		"sqlite v1 chunk key for file tag {file_tag} had invalid length {}",
		key.len()
	);

	Ok(u32::from_be_bytes(
		key[prefix.len()..]
			.try_into()
			.expect("sqlite v1 chunk key index bytes should exist"),
	))
}

fn split_bytes(bytes: &[u8], max_chunk_bytes: usize) -> Vec<Vec<u8>> {
	if bytes.is_empty() || max_chunk_bytes == 0 {
		return vec![bytes.to_vec()];
	}

	bytes
		.chunks(max_chunk_bytes)
		.map(|chunk| chunk.to_vec())
		.collect()
}

fn v1_meta_key(file_tag: u8) -> [u8; 4] {
	[
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_META_PREFIX,
		file_tag,
	]
}

fn v1_chunk_prefix(file_tag: u8) -> [u8; 4] {
	[
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_CHUNK_PREFIX,
		file_tag,
	]
}

#[cfg(test)]
fn v1_chunk_key(file_tag: u8, chunk_idx: u32) -> [u8; 8] {
	let chunk_idx = chunk_idx.to_be_bytes();
	[
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_CHUNK_PREFIX,
		file_tag,
		chunk_idx[0],
		chunk_idx[1],
		chunk_idx[2],
		chunk_idx[3],
	]
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
	if let Some(meta) = sqlite_engine.try_load_meta(&actor_id).await? {
		ensure!(
			!matches!(meta.origin, SqliteOrigin::MigratingFromV1),
			"sqlite v1 migration for actor {actor_id} is incomplete"
		);
	}
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

struct V1File {
	size_bytes: u64,
	bytes: Vec<u8>,
}

struct RecoveredV1Snapshot {
	main: V1File,
	journal: Option<V1File>,
	had_journal: bool,
}

struct RecoveredDb {
	bytes: Vec<u8>,
	total_pages: u32,
	had_journal: bool,
}

#[cfg(test)]
mod tests {
	use std::path::Path;
	use std::sync::Arc;

	use anyhow::Result;
	use gas::prelude::{Id, util::timestamp};
	use pegboard::actor_kv::Recipient;
	use rusqlite::{Connection, params};
	use sqlite_storage::{
		commit::{CommitRequest, CommitStageRequest},
		engine::SqliteEngine,
		keys::meta_key,
		ltx::{LtxHeader, encode_ltx_v3},
		takeover::TakeoverConfig,
		types::{DirtyPage, SqliteOrigin},
		udb::{WriteOp, apply_write_ops},
	};
	use tempfile::tempdir;
	use universaldb::driver::RocksDbDatabaseDriver;

	use pegboard::actor_sqlite_v2::sqlite_subspace;

	use super::{
		FILE_TAG_JOURNAL, FILE_TAG_MAIN, FILE_TAG_SHM, FILE_TAG_WAL, SQLITE_V1_CHUNK_SIZE,
		SQLITE_V1_MAX_MIGRATION_BYTES, SQLITE_V1_MIGRATION_LEASE_MS, maybe_migrate_v1_to_v2,
		read_v1_file, v1_chunk_key, v1_meta_key,
	};

	fn recipient(actor_id: Id) -> Recipient {
		Recipient {
			actor_id,
			namespace_id: Id::new_v1(1),
			name: "test".to_string(),
		}
	}

	async fn test_db() -> Result<Arc<universaldb::Database>> {
		let path = tempdir()?.keep();
		let driver = RocksDbDatabaseDriver::new(path).await?;
		Ok(Arc::new(universaldb::Database::new(Arc::new(driver))))
	}

	fn sqlite_file_bytes(path: &Path) -> Result<Vec<u8>> {
		Ok(std::fs::read(path)?)
	}

	fn configure_v1_pragmas_with_page_size(conn: &Connection, page_size: u32) -> Result<()> {
		conn.pragma_update(None, "page_size", page_size)?;
		conn.pragma_update(None, "journal_mode", "DELETE")?;
		conn.pragma_update(None, "synchronous", "NORMAL")?;
		conn.pragma_update(None, "temp_store", "MEMORY")?;
		conn.pragma_update(None, "auto_vacuum", "NONE")?;
		conn.pragma_update(None, "locking_mode", "EXCLUSIVE")?;
		Ok(())
	}

	fn configure_v1_pragmas(conn: &Connection) -> Result<()> {
		configure_v1_pragmas_with_page_size(conn, 4096)
	}

	fn encode_v1_meta(size: u64) -> [u8; 10] {
		let mut bytes = [0_u8; 10];
		bytes[..2].copy_from_slice(&1_u16.to_le_bytes());
		bytes[2..].copy_from_slice(&size.to_le_bytes());
		bytes
	}

	async fn seed_v1_file(
		db: &universaldb::Database,
		recipient: &Recipient,
		file_tag: u8,
		bytes: &[u8],
	) -> Result<()> {
		let mut keys = vec![v1_meta_key(file_tag).to_vec()];
		let mut values = vec![encode_v1_meta(bytes.len() as u64).to_vec()];
		for (chunk_idx, chunk) in bytes.chunks(SQLITE_V1_CHUNK_SIZE).enumerate() {
			if keys.len() == 128 {
				pegboard::actor_kv::put(
					db,
					recipient,
					std::mem::take(&mut keys),
					std::mem::take(&mut values),
				)
				.await?;
			}
			keys.push(v1_chunk_key(file_tag, chunk_idx as u32).to_vec());
			values.push(chunk.to_vec());
		}
		pegboard::actor_kv::put(db, recipient, keys, values).await
	}

	async fn seed_v1_sparse_chunks(
		db: &universaldb::Database,
		recipient: &Recipient,
		file_tag: u8,
		size_bytes: u64,
		chunk_count: u32,
	) -> Result<()> {
		let mut keys = vec![v1_meta_key(file_tag).to_vec()];
		let mut values = vec![encode_v1_meta(size_bytes).to_vec()];
		for chunk_idx in 0..chunk_count {
			if keys.len() == 128 {
				pegboard::actor_kv::put(
					db,
					recipient,
					std::mem::take(&mut keys),
					std::mem::take(&mut values),
				)
				.await?;
			}
			keys.push(v1_chunk_key(file_tag, chunk_idx).to_vec());
			values.push(vec![(chunk_idx as u8).wrapping_add(1)]);
		}
		pegboard::actor_kv::put(db, recipient, keys, values).await
	}

	async fn age_v1_migration_head(
		db: &universaldb::Database,
		engine: &SqliteEngine,
		actor_id: &str,
	) -> Result<()> {
		let mut head = engine.load_head(actor_id).await?;
		head.creation_ts_ms -= SQLITE_V1_MIGRATION_LEASE_MS + 1;
		apply_write_ops(
			db,
			&sqlite_subspace(),
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(actor_id), serde_bare::to_vec(&head)?)],
		)
		.await
	}

	async fn load_v2_bytes(engine: &SqliteEngine, actor_id: &str) -> Result<Vec<u8>> {
		let meta = engine.load_meta(actor_id).await?;
		let pages = engine
			.get_pages(
				actor_id,
				meta.generation,
				(1..=meta.db_size_pages).collect(),
			)
			.await?;
		let mut bytes = Vec::with_capacity(meta.db_size_pages as usize * meta.page_size as usize);
		for page in pages {
			bytes.extend_from_slice(
				&page
					.bytes
					.unwrap_or_else(|| vec![0; meta.page_size as usize]),
			);
		}
		Ok(bytes)
	}

	fn query_note_values(bytes: &[u8]) -> Result<Vec<String>> {
		let tmp = tempdir()?;
		let path = tmp.path().join("query.db");
		std::fs::write(&path, bytes)?;
		let conn = Connection::open(path)?;
		let mut stmt = conn.prepare("SELECT note FROM items ORDER BY id")?;
		let values = stmt
			.query_map([], |row| row.get::<_, String>(0))?
			.collect::<std::result::Result<Vec<_>, _>>()?;
		let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
		assert_eq!(integrity, "ok");
		Ok(values)
	}

	fn build_fixture_db(notes: &[&str]) -> Result<Vec<u8>> {
		let tmp = tempdir()?;
		let path = tmp.path().join("fixture.db");
		let conn = Connection::open(&path)?;
		configure_v1_pragmas(&conn)?;
		conn.execute_batch(
			"CREATE TABLE items (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
			 CREATE INDEX idx_items_note ON items(note);",
		)?;
		let tx = conn.unchecked_transaction()?;
		for note in notes {
			tx.execute("INSERT INTO items(note) VALUES (?1)", params![note])?;
		}
		tx.commit()?;
		drop(conn);
		sqlite_file_bytes(&path)
	}

	fn build_fixture_db_with_page_size(notes: &[&str], page_size: u32) -> Result<Vec<u8>> {
		let tmp = tempdir()?;
		let path = tmp.path().join("fixture.db");
		let conn = Connection::open(&path)?;
		configure_v1_pragmas_with_page_size(&conn, page_size)?;
		conn.execute_batch("CREATE TABLE items (id INTEGER PRIMARY KEY, note TEXT NOT NULL);")?;
		let tx = conn.unchecked_transaction()?;
		for note in notes {
			tx.execute("INSERT INTO items(note) VALUES (?1)", params![note])?;
		}
		tx.commit()?;
		drop(conn);
		sqlite_file_bytes(&path)
	}

	fn build_open_tx_fixture() -> Result<(Vec<u8>, Vec<u8>)> {
		let tmp = tempdir()?;
		let path = tmp.path().join("fixture.db");
		let conn = Connection::open(&path)?;
		configure_v1_pragmas(&conn)?;
		conn.execute_batch("CREATE TABLE items (id INTEGER PRIMARY KEY, note TEXT NOT NULL);")?;
		conn.execute("INSERT INTO items(note) VALUES (?1)", params!["before"])?;
		conn.execute_batch("BEGIN IMMEDIATE;")?;
		conn.execute("INSERT INTO items(note) VALUES (?1)", params!["during"])?;
		let main = sqlite_file_bytes(&path)?;
		let journal = sqlite_file_bytes(&tmp.path().join("fixture.db-journal"))?;
		Ok((main, journal))
	}

	#[tokio::test]
	async fn migrates_v1_sqlite_into_v2_storage() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db(&["alpha", "beta", "gamma", "delta"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		assert!(maybe_migrate_v1_to_v2(&db, &engine, &recipient).await?);

		let meta = engine.load_meta(&actor_id.to_string()).await?;
		assert!(meta.migrated_from_v1);
		assert_eq!(meta.origin, SqliteOrigin::MigratedFromV1);
		assert_eq!(
			query_note_values(&load_v2_bytes(&engine, &actor_id.to_string()).await?)?,
			vec!["alpha", "beta", "gamma", "delta"]
		);

		Ok(())
	}

	#[tokio::test]
	async fn retries_cleanly_after_stale_partial_v1_import() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db(&["retry-a", "retry-b", "retry-c"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());
		let actor_id_str = actor_id.to_string();

		let prepared = engine
			.prepare_v1_migration(&actor_id_str, timestamp::now())
			.await?;
		let stage = engine
			.commit_stage_begin(
				&actor_id_str,
				sqlite_storage::commit::CommitStageBeginRequest {
					generation: prepared.meta.generation,
				},
			)
			.await?;
		let dirty_pages = fixture
			.chunks(super::SQLITE_V1_CHUNK_SIZE)
			.enumerate()
			.map(|(idx, bytes)| DirtyPage {
				pgno: idx as u32 + 1,
				bytes: bytes.to_vec(),
			})
			.collect::<Vec<_>>();
		let encoded = encode_ltx_v3(
			LtxHeader::delta(stage.txid, dirty_pages.len() as u32, timestamp::now()),
			&dirty_pages,
		)?;
		engine
			.commit_stage(
				&actor_id_str,
				CommitStageRequest {
					generation: prepared.meta.generation,
					txid: stage.txid,
					chunk_idx: 0,
					bytes: encoded,
					is_last: true,
				},
			)
			.await?;
		age_v1_migration_head(&db, &engine, &actor_id_str).await?;

		assert!(maybe_migrate_v1_to_v2(&db, &engine, &recipient).await?);
		let meta = engine.load_meta(&actor_id_str).await?;
		assert_eq!(meta.origin, SqliteOrigin::MigratedFromV1);
		assert_eq!(
			query_note_values(&load_v2_bytes(&engine, &actor_id_str).await?)?,
			vec!["retry-a", "retry-b", "retry-c"]
		);

		Ok(())
	}

	#[tokio::test]
	async fn rejects_fresh_in_progress_v1_migrations() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db(&["fresh-retry-a", "fresh-retry-b"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());
		let actor_id_str = actor_id.to_string();

		let prepared = engine
			.prepare_v1_migration(&actor_id_str, timestamp::now())
			.await?;
		engine
			.commit_stage_begin(
				&actor_id_str,
				sqlite_storage::commit::CommitStageBeginRequest {
					generation: prepared.meta.generation,
				},
			)
			.await?;

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("fresh staged migration should not be retried");
		assert!(
			err.to_string().contains("already in progress"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn skips_native_v2_state_even_if_v1_tombstone_exists() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let actor_id_str = actor_id.to_string();
		let v1_fixture = build_fixture_db(&["legacy"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &v1_fixture).await?;
		let native_fixture = build_fixture_db(&["native"])?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());
		let takeover = engine
			.takeover(&actor_id_str, TakeoverConfig::new(timestamp::now()))
			.await?;
		let dirty_pages = native_fixture
			.chunks(super::SQLITE_V1_CHUNK_SIZE)
			.enumerate()
			.map(|(idx, bytes)| DirtyPage {
				pgno: idx as u32 + 1,
				bytes: bytes.to_vec(),
			})
			.collect::<Vec<_>>();
		engine
			.commit(
				&actor_id_str,
				CommitRequest {
					generation: takeover.generation,
					head_txid: takeover.meta.head_txid,
					db_size_pages: dirty_pages.len() as u32,
					dirty_pages,
					now_ms: timestamp::now(),
				},
			)
			.await?;

		assert!(!maybe_migrate_v1_to_v2(&db, &engine, &recipient).await?);

		let meta = engine.load_meta(&actor_id_str).await?;
		assert_eq!(meta.origin, SqliteOrigin::Native);
		assert_eq!(
			query_note_values(&load_v2_bytes(&engine, &actor_id_str).await?)?,
			vec!["native"]
		);

		Ok(())
	}

	#[tokio::test]
	async fn bails_when_v2_meta_is_unreadable() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let actor_id_str = actor_id.to_string();
		let fixture = build_fixture_db(&["broken-meta"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());
		apply_write_ops(
			db.as_ref(),
			&sqlite_subspace(),
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(&actor_id_str),
				b"not-a-db-head".to_vec(),
			)],
		)
		.await?;

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("corrupt meta should fail migration");
		assert!(
			err.to_string().contains("decode sqlite db head"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn recovers_a_pending_v1_journal_before_import() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let (main, journal) = build_open_tx_fixture()?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &main).await?;
		seed_v1_file(&db, &recipient, FILE_TAG_JOURNAL, &journal).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		assert!(maybe_migrate_v1_to_v2(&db, &engine, &recipient).await?);
		assert_eq!(
			query_note_values(&load_v2_bytes(&engine, &actor_id.to_string()).await?)?,
			vec!["before"]
		);

		Ok(())
	}

	#[tokio::test]
	async fn migrates_zero_size_v1_state_without_pages() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &[]).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		assert!(maybe_migrate_v1_to_v2(&db, &engine, &recipient).await?);

		let meta = engine.load_meta(&actor_id.to_string()).await?;
		assert_eq!(meta.origin, SqliteOrigin::MigratedFromV1);
		assert_eq!(meta.db_size_pages, 0);
		assert!(
			load_v2_bytes(&engine, &actor_id.to_string())
				.await?
				.is_empty()
		);

		Ok(())
	}

	#[tokio::test]
	async fn rejects_v1_databases_with_unsupported_page_size() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db_with_page_size(&["wrong-page-size"], 8192)?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("unsupported page size should fail migration");
		assert!(
			err.to_string()
				.contains("sqlite page size 8192 is not supported by sqlite v2"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn rejects_v1_wal_sidecars() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db(&["wal-sidecar"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		seed_v1_file(&db, &recipient, FILE_TAG_WAL, b"unexpected wal bytes").await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("wal sidecar should fail migration");
		assert!(
			err.to_string()
				.contains("unexpected sqlite v1 WAL sidecar present"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn rejects_v1_shm_sidecars() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db(&["shm-sidecar"])?;
		seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
		seed_v1_file(&db, &recipient, FILE_TAG_SHM, b"unexpected shm bytes").await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("shm sidecar should fail migration");
		assert!(
			err.to_string()
				.contains("unexpected sqlite v1 SHM sidecar present"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn rejects_v1_files_with_missing_chunks() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let fixture = build_fixture_db(&["chunk-a", "chunk-b", "chunk-c", "chunk-d"])?;
		let mut keys = vec![v1_meta_key(FILE_TAG_MAIN).to_vec()];
		let mut values = vec![encode_v1_meta(fixture.len() as u64).to_vec()];
		for (chunk_idx, chunk) in fixture.chunks(SQLITE_V1_CHUNK_SIZE).enumerate() {
			if chunk_idx == 1 {
				continue;
			}
			if keys.len() == 128 {
				pegboard::actor_kv::put(
					&db,
					&recipient,
					std::mem::take(&mut keys),
					std::mem::take(&mut values),
				)
				.await?;
			}
			keys.push(v1_chunk_key(FILE_TAG_MAIN, chunk_idx as u32).to_vec());
			values.push(chunk.to_vec());
		}
		pegboard::actor_kv::put(&db, &recipient, keys, values).await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("missing chunk should fail migration");
		let err_debug = format!("{err:?}");
		assert!(
			err_debug.contains("sqlite v1 file expected")
				|| err_debug.contains("missing or duplicated chunk"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn rejects_v1_files_that_exceed_migration_limit() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		pegboard::actor_kv::put(
			&db,
			&recipient,
			vec![v1_meta_key(FILE_TAG_MAIN).to_vec()],
			vec![encode_v1_meta(SQLITE_V1_MAX_MIGRATION_BYTES + 1).to_vec()],
		)
		.await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), sqlite_subspace());

		let err = maybe_migrate_v1_to_v2(&db, &engine, &recipient)
			.await
			.expect_err("oversized v1 file should fail migration");
		assert!(
			err.to_string().contains("exceeded migration limit"),
			"unexpected error: {err:?}"
		);

		Ok(())
	}

	#[tokio::test]
	async fn reads_v1_files_beyond_the_default_kv_list_limit() -> Result<()> {
		let db = test_db().await?;
		let actor_id = Id::new_v1(1);
		let recipient = recipient(actor_id);
		let chunk_count = 16_385_u32;
		let size_bytes = u64::from(chunk_count) * SQLITE_V1_CHUNK_SIZE as u64;
		seed_v1_sparse_chunks(&db, &recipient, FILE_TAG_MAIN, size_bytes, chunk_count).await?;

		let file = read_v1_file(&db, &recipient, FILE_TAG_MAIN)
			.await?
			.expect("sparse v1 file should exist");
		assert_eq!(file.size_bytes, size_bytes);
		assert_eq!(file.bytes.len(), size_bytes as usize);
		assert_eq!(file.bytes[0], 1);
		assert_eq!(file.bytes[SQLITE_V1_CHUNK_SIZE], 2);
		assert_eq!(
			file.bytes[(chunk_count as usize - 1) * SQLITE_V1_CHUNK_SIZE],
			(chunk_count as u8).wrapping_add(0)
		);

		Ok(())
	}
}
