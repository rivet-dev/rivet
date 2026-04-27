use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use gas::prelude::{Id, util::timestamp};
use pegboard::actor_kv::Recipient;
use rusqlite::{Connection, params};
use sqlite_storage::{
	commit::{CommitRequest, CommitStageBeginRequest, CommitStageRequest},
	engine::SqliteEngine,
	keys::meta_key,
	ltx::{LtxHeader, encode_ltx_v3},
	open::OpenConfig,
	types::{DirtyPage, SqliteOrigin},
	udb::{self, WriteOp},
};
use tempfile::tempdir;
use universaldb::driver::RocksDbDatabaseDriver;

const SQLITE_V1_PREFIX: u8 = 0x08;
const SQLITE_V1_SCHEMA_VERSION: u8 = 0x01;
const SQLITE_V1_META_PREFIX: u8 = 0x00;
const SQLITE_V1_CHUNK_PREFIX: u8 = 0x01;
const SQLITE_V1_CHUNK_SIZE: usize = 4096;
const SQLITE_V1_MAX_MIGRATION_BYTES: u64 = 128 * 1024 * 1024;
const SQLITE_V1_MIGRATION_LEASE_MS: i64 = 60 * 1000;
const FILE_TAG_MAIN: u8 = 0x00;
const FILE_TAG_JOURNAL: u8 = 0x01;
const FILE_TAG_WAL: u8 = 0x02;
const FILE_TAG_SHM: u8 = 0x03;

fn recipient(actor_id: Id) -> Recipient {
	Recipient {
		actor_id,
		namespace_id: Id::new_v1(1),
		name: "test".to_string(),
	}
}

async fn test_db() -> Result<universaldb::Database> {
	let path = tempdir()?.keep();
	let driver = RocksDbDatabaseDriver::new(path).await?;
	Ok(universaldb::Database::new(Arc::new(driver)))
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

fn v1_meta_key(file_tag: u8) -> Vec<u8> {
	vec![
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_META_PREFIX,
		file_tag,
	]
}

fn v1_chunk_key(file_tag: u8, chunk_idx: u32) -> Vec<u8> {
	let mut key = vec![
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_CHUNK_PREFIX,
		file_tag,
	];
	key.extend_from_slice(&chunk_idx.to_be_bytes());
	key
}

async fn seed_v1_file(
	db: &universaldb::Database,
	recipient: &Recipient,
	file_tag: u8,
	bytes: &[u8],
) -> Result<()> {
	let mut keys = vec![v1_meta_key(file_tag)];
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
		keys.push(v1_chunk_key(file_tag, chunk_idx as u32));
		values.push(chunk.to_vec());
	}
	pegboard::actor_kv::put(db, recipient, keys, values).await
}

async fn migrate(
	db: &universaldb::Database,
	actor_id: Id,
) -> Result<pegboard::actor_sqlite::MigrateV1ToV2Output> {
	pegboard::actor_sqlite::migrate_v1_to_v2(
		db.clone(),
		pegboard::actor_sqlite::MigrateV1ToV2Input {
			actor_id,
			namespace_id: Id::new_v1(1),
			name: "test".to_string(),
		},
	)
	.await
}

async fn age_v1_migration_head(
	db: &universaldb::Database,
	engine: &SqliteEngine,
	actor_id: &str,
) -> Result<()> {
	let mut head = engine.load_head(actor_id).await?;
	head.creation_ts_ms -= SQLITE_V1_MIGRATION_LEASE_MS + 1;
	udb::apply_write_ops(
		db,
		&pegboard::actor_sqlite::sqlite_subspace(),
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

	assert!(migrate(&db, actor_id).await?.migrated);

	let (engine, _compaction_rx) = pegboard::actor_sqlite::new_engine(db.clone());
	let actor_id_str = actor_id.to_string();
	let meta = engine.load_meta(&actor_id_str).await?;
	assert_eq!(meta.origin, SqliteOrigin::MigratedFromV1);
	assert_eq!(
		query_note_values(&load_v2_bytes(&engine, &actor_id_str).await?)?,
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
	let (engine, _compaction_rx) = pegboard::actor_sqlite::new_engine(db.clone());
	let actor_id_str = actor_id.to_string();

	let prepared = engine
		.prepare_v1_migration(&actor_id_str, timestamp::now())
		.await?;
	let stage = engine
		.commit_stage_begin(
			&actor_id_str,
			CommitStageBeginRequest {
				generation: prepared.meta.generation,
			},
		)
		.await?;
	let dirty_pages = fixture
		.chunks(SQLITE_V1_CHUNK_SIZE)
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

	assert!(migrate(&db, actor_id).await?.migrated);
	let meta = engine.load_meta(&actor_id_str).await?;
	assert_eq!(meta.origin, SqliteOrigin::MigratedFromV1);
	assert_eq!(
		query_note_values(&load_v2_bytes(&engine, &actor_id_str).await?)?,
		vec!["retry-a", "retry-b", "retry-c"]
	);

	Ok(())
}

#[tokio::test]
async fn restarts_v1_migration_after_allocate_invalidation() -> Result<()> {
	let db = test_db().await?;
	let actor_id = Id::new_v1(1);
	let recipient = recipient(actor_id);
	let fixture = build_fixture_db(&["allocate-retry-a", "allocate-retry-b"])?;
	seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;
	let (engine, _compaction_rx) = pegboard::actor_sqlite::new_engine(db.clone());
	let actor_id_str = actor_id.to_string();

	let prepared = engine
		.prepare_v1_migration(&actor_id_str, timestamp::now())
		.await?;
	engine
		.commit_stage_begin(
			&actor_id_str,
			CommitStageBeginRequest {
				generation: prepared.meta.generation,
			},
		)
		.await?;

	assert!(migrate(&db, actor_id).await?.migrated);
	assert_eq!(
		query_note_values(&load_v2_bytes(&engine, &actor_id_str).await?)?,
		vec!["allocate-retry-a", "allocate-retry-b"]
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
	let (engine, _compaction_rx) = pegboard::actor_sqlite::new_engine(db.clone());
	let opened = engine
		.open(&actor_id_str, OpenConfig::new(timestamp::now()))
		.await?;
	let dirty_pages = native_fixture
		.chunks(SQLITE_V1_CHUNK_SIZE)
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
				generation: opened.generation,
				head_txid: opened.meta.head_txid,
				db_size_pages: dirty_pages.len() as u32,
				dirty_pages,
				now_ms: timestamp::now(),
			},
		)
		.await?;

	assert!(!migrate(&db, actor_id).await?.migrated);

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
	let (engine, _compaction_rx) = pegboard::actor_sqlite::new_engine(db.clone());
	udb::apply_write_ops(
		&db,
		&pegboard::actor_sqlite::sqlite_subspace(),
		engine.op_counter.as_ref(),
		vec![WriteOp::put(
			meta_key(&actor_id_str),
			b"not-a-db-head".to_vec(),
		)],
	)
	.await?;

	let err = migrate(&db, actor_id)
		.await
		.expect_err("corrupt meta should fail migration");
	assert!(
		err.to_string().contains("decode sqlite db head"),
		"unexpected error: {err:?}"
	);

	Ok(())
}

#[tokio::test]
async fn rejects_v1_journal_sidecars() -> Result<()> {
	let db = test_db().await?;
	let actor_id = Id::new_v1(1);
	let recipient = recipient(actor_id);
	let (main, journal) = build_open_tx_fixture()?;
	seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &main).await?;
	seed_v1_file(&db, &recipient, FILE_TAG_JOURNAL, &journal).await?;

	let err = migrate(&db, actor_id)
		.await
		.expect_err("journal sidecar should fail migration");
	let msg = err.to_string();
	assert!(
		msg.contains("crashed during a write transaction"),
		"unexpected error: {err:?}"
	);
	assert!(
		msg.contains(&actor_id.to_string()),
		"error should include actor id: {err:?}"
	);

	Ok(())
}

#[tokio::test]
async fn migrates_zero_size_v1_state_without_pages() -> Result<()> {
	let db = test_db().await?;
	let actor_id = Id::new_v1(1);
	let recipient = recipient(actor_id);
	seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &[]).await?;

	assert!(migrate(&db, actor_id).await?.migrated);

	let (engine, _compaction_rx) = pegboard::actor_sqlite::new_engine(db.clone());
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
async fn rejects_v1_main_with_corrupt_magic_byte() -> Result<()> {
	let db = test_db().await?;
	let actor_id = Id::new_v1(1);
	let recipient = recipient(actor_id);
	let mut fixture = build_fixture_db(&["needs-magic"])?;
	// Flip a byte in the SQLite magic header. rusqlite would have refused
	// to open this file at all; in the simplified path the in-memory header
	// validation is the only line of defense, so this test pins it down.
	fixture[0] = b'X';
	seed_v1_file(&db, &recipient, FILE_TAG_MAIN, &fixture).await?;

	let err = migrate(&db, actor_id)
		.await
		.expect_err("corrupt magic should fail migration");
	assert!(
		err.to_string().contains("magic bytes mismatch"),
		"unexpected error: {err:?}"
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

	let err = migrate(&db, actor_id)
		.await
		.expect_err("unsupported page size should fail migration");
	assert!(
		err.to_string()
			.contains("sqlite v1 page size 8192 is not supported"),
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

	let err = migrate(&db, actor_id)
		.await
		.expect_err("wal sidecar should fail migration");
	assert!(
		err.to_string().contains("unexpected WAL sidecar"),
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

	let err = migrate(&db, actor_id)
		.await
		.expect_err("shm sidecar should fail migration");
	assert!(
		err.to_string().contains("unexpected SHM sidecar"),
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
	let mut keys = vec![v1_meta_key(FILE_TAG_MAIN)];
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
		keys.push(v1_chunk_key(FILE_TAG_MAIN, chunk_idx as u32));
		values.push(chunk.to_vec());
	}
	pegboard::actor_kv::put(&db, &recipient, keys, values).await?;

	let err = migrate(&db, actor_id)
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
		vec![v1_meta_key(FILE_TAG_MAIN)],
		vec![encode_v1_meta(SQLITE_V1_MAX_MIGRATION_BYTES + 1).to_vec()],
	)
	.await?;

	let err = migrate(&db, actor_id)
		.await
		.expect_err("oversized v1 file should fail migration");
	assert!(
		err.to_string().contains("exceeded migration limit"),
		"unexpected error: {err:?}"
	);

	Ok(())
}
