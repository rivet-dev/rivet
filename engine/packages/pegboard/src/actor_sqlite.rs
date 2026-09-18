use std::{sync::Arc, time::Instant};

use anyhow::{Context, Result, ensure};
use depot::{
	conveyer::{Db, branch as depot_branch},
	keys as depot_keys,
	types::{BucketId, CommitOptions, DBHead, DirtyPage, SQLITE_PAGE_SIZE, decode_db_head},
};
use gas::prelude::{Id, util::timestamp};
use rivet_envoy_protocol as protocol;
use rivet_pools::NodeId;

use crate::{actor_kv::Recipient, metrics};

const SQLITE_V1_PREFIX: u8 = 0x08;
const SQLITE_V1_SCHEMA_VERSION: u8 = 0x01;
const SQLITE_V1_META_PREFIX: u8 = 0x00;
const SQLITE_V1_CHUNK_PREFIX: u8 = 0x01;
const SQLITE_V1_META_VERSION: u16 = 1;
const SQLITE_V1_META_LEN: usize = 10;
const SQLITE_V1_CHUNK_SIZE: usize = 4096;
const SQLITE_V1_MAX_MIGRATION_BYTES: u64 = 128 * 1024 * 1024;
/// Pages written per commit while importing a v1 database, so a v1 database larger
/// than one commit is imported across several commits.
///
/// Deliberately its own value rather than depot's commit cap. The two answer
/// different questions: the cap is the largest commit depot will accept, this is
/// how coarsely the importer should batch. The import writes through the
/// single-shot commit path, so the batch has to stay small enough to fit one FDB
/// transaction, and it also bounds how much work an interrupted import repeats.
pub const MIGRATION_COMMIT_PAGES: usize = 320;
const FILE_TAG_MAIN: u8 = 0x00;
const FILE_TAG_JOURNAL: u8 = 0x01;
const FILE_TAG_WAL: u8 = 0x02;
const FILE_TAG_SHM: u8 = 0x03;
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

pub fn clear_v2_storage_for_destroy(tx: &universaldb::Transaction, actor_id: Id) {
	tx.informal().clear(&migration_marker_key(actor_id));

	let actor_id = actor_id.to_string();

	tx.informal().clear(&depot_keys::meta_head_key(&actor_id));
	tx.informal()
		.clear(&depot_keys::meta_compact_key(&actor_id));
	tx.informal().clear(&depot_keys::meta_quota_key(&actor_id));
	// Clear the lease with the rest of Depot.
	// Otherwise dead lease keys accumulate in UDB indefinitely.
	tx.informal()
		.clear(&depot_keys::meta_compactor_lease_key(&actor_id));

	for prefix in [
		depot_keys::shard_prefix(&actor_id),
		depot_keys::delta_prefix(&actor_id),
		depot_keys::pidx_delta_prefix(&actor_id),
	] {
		let (begin, end) = prefix_range(&prefix);
		tx.informal().clear_range(&begin, &end);
	}
}

fn migration_marker_key(actor_id: Id) -> Vec<u8> {
	crate::keys::subspace().pack(&crate::keys::actor::SqliteMigrationKey::new(actor_id))
}

fn prefix_range(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
	universaldb::tuple::Subspace::from_bytes(prefix.to_vec()).range()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Hash)]
pub struct MigrateV1ToV2Input {
	pub actor_id: Id,
	pub namespace_id: Id,
	pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Hash)]
pub struct MigrateV1ToV2Output {
	pub migrated: bool,
}

pub async fn migrate_v1_to_v2(
	db: universaldb::Database,
	config: rivet_config::Config,
	input: MigrateV1ToV2Input,
) -> Result<MigrateV1ToV2Output> {
	let recipient = Recipient {
		actor_id: input.actor_id,
		namespace_id: input.namespace_id,
		name: input.name,
	};

	let migrated = maybe_migrate_v1_to_v2(&db, &config, &recipient).await?;

	Ok(MigrateV1ToV2Output { migrated })
}

async fn maybe_migrate_v1_to_v2(
	db: &universaldb::Database,
	config: &rivet_config::Config,
	recipient: &Recipient,
) -> Result<bool> {
	if !crate::actor_kv::sqlite_v1_data_exists(db, recipient.actor_id).await? {
		return Ok(false);
	}

	let actor_id = recipient.actor_id.to_string();

	let state = load_migration_state(db, recipient).await?;

	// A v2 head with no in-progress marker is a complete database, either
	// because an earlier migration finished or because the actor already ran on
	// v2. The v1 data is never deleted, so this is the steady state for every
	// already-migrated actor.
	if state.head.is_some() && state.in_progress_pages.is_none() {
		return Ok(false);
	}

	metrics::SQLITE_MIGRATION_ATTEMPTS_TOTAL.inc();
	let start = Instant::now();

	// Known failure case: if the v1 main file is corrupt or otherwise
	// unparseable (for example a torn write left by a crash mid-transaction),
	// reading or validating it returns a deterministic error. That error
	// propagates out of the activity, so the activity fails and keeps retrying
	// without making progress. There is no automatic recovery for this. The
	// actor stays on v1 and requires manual triage to recover or discard its
	// database.
	let main = read_v1_main(db, recipient)
		.await
		.map_err(|err| migration_error(&actor_id, "read_v1", err))?;
	let recovered = validate_v1_main(&actor_id, main)
		.map_err(|err| migration_error(&actor_id, "validate", err))?;
	metrics::SQLITE_MIGRATION_PAGES.observe(recovered.total_pages as f64);
	tracing::info!(
		actor_id = %actor_id,
		pages = recovered.total_pages,
		size_bytes = recovered.bytes.len(),
		"starting v1→v2 migration"
	);

	let actor_db = Db::new(
		Arc::new(db.clone()),
		recipient.namespace_id,
		actor_id.clone(),
		NodeId::new(),
	)
	.with_config(config.clone());

	// The marker is written before the first commit so that a crash part way
	// through the import is always recognizable as an unfinished import rather
	// than as a complete database.
	write_migration_marker(db, recipient.actor_id, Some(recovered.total_pages))
		.await
		.map_err(|err| migration_error(&actor_id, "mark", err))?;

	// Only resume on top of a head left by an unfinished import of this same
	// database. A recorded page count that disagrees with what the v1 data reads
	// as now means the published pages cannot be assumed to match, so the import
	// starts over.
	let resume_head = state.head.as_ref().filter(|_| {
		state
			.in_progress_pages
			.is_some_and(|pages| pages == recovered.total_pages)
	});
	// The fence follows whatever head is actually published, even when the import
	// starts over, so the first commit is not rejected for expecting no head.
	let import = ImportPlan {
		fence_head_txid: state.head.as_ref().map_or(0, |head| head.head_txid),
		start_pgno: resume_head.map_or(2, |head| head.db_size_pages.saturating_add(1).max(2)),
	};

	import_v1_pages(&actor_db, &actor_id, &recovered, import)
		.await
		.map_err(|err| migration_error(&actor_id, "finalize", err))?;

	write_migration_marker(db, recipient.actor_id, None)
		.await
		.map_err(|err| migration_error(&actor_id, "unmark", err))?;

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

/// Imports the recovered v1 database into v2 storage.
///
/// Depot caps how much a single commit may carry, so anything but a tiny
/// database is imported across several commits. Two properties keep a failure
/// part way through from producing a database that looks complete:
///
/// - Page 1 holds the SQLite header and is committed last, so a partial import
///   has no header and cannot be opened as a valid database.
/// - Every commit fences on the head txid it expects, so a second concurrent
///   attempt fails instead of interleaving its pages with this one.
///
/// A retry resumes from the published head because the v1 source is immutable
/// for the lifetime of the migration, which makes the pages already written
/// byte identical to what a fresh import would write.
async fn import_v1_pages(
	actor_db: &Db,
	actor_id: &str,
	recovered: &RecoveredDb,
	import: ImportPlan,
) -> Result<()> {
	let total_pages = recovered.total_pages;
	let mut expected_head_txid = import.fence_head_txid;

	if total_pages == 0 {
		commit_pages(actor_db, Vec::new(), 0, &mut expected_head_txid).await?;
		return Ok(());
	}

	// Pages already published by an earlier attempt do not need to be rewritten.
	// Page 1 is never among them because it is only committed once every other
	// page has landed.
	let mut next_pgno = import.start_pgno;
	if next_pgno > 2 {
		tracing::info!(
			actor_id = %actor_id,
			fence_head_txid = import.fence_head_txid,
			next_pgno,
			total_pages,
			"resuming unfinished v1→v2 import"
		);
	}

	while next_pgno <= total_pages {
		let end_pgno = next_pgno
			.saturating_add(u32::try_from(MIGRATION_COMMIT_PAGES)? - 1)
			.min(total_pages);
		let pages = (next_pgno..=end_pgno)
			.map(|pgno| page_at(recovered, pgno))
			.collect::<Result<Vec<_>>>()?;

		// The intermediate size is the highest page written so far, which keeps
		// every page in the commit within the database and avoids publishing a
		// size that references pages that do not exist yet.
		commit_pages(actor_db, pages, end_pgno, &mut expected_head_txid).await?;

		next_pgno = end_pgno.saturating_add(1);
	}

	// Publishing page 1 with the real page count completes the database.
	commit_pages(
		actor_db,
		vec![page_at(recovered, 1)?],
		total_pages,
		&mut expected_head_txid,
	)
	.await?;

	Ok(())
}

struct ImportPlan {
	/// Head txid the first commit of the import must observe.
	fence_head_txid: u64,
	/// First page to write. Always at least 2, since page 1 is written last.
	start_pgno: u32,
}

async fn commit_pages(
	actor_db: &Db,
	pages: Vec<DirtyPage>,
	db_size_pages: u32,
	expected_head_txid: &mut u64,
) -> Result<()> {
	let result = actor_db
		.commit_with_options(
			pages,
			db_size_pages,
			timestamp::now(),
			CommitOptions {
				expected_head_txid: Some(*expected_head_txid),
				disable_size_cap: false,
			},
		)
		.await?;
	*expected_head_txid = result.head_txid;

	Ok(())
}

fn page_at(recovered: &RecoveredDb, pgno: u32) -> Result<DirtyPage> {
	let page_size = SQLITE_PAGE_SIZE as usize;
	let start = (pgno as usize - 1)
		.checked_mul(page_size)
		.context("sqlite v1 page offset overflow")?;
	let bytes = recovered
		.bytes
		.get(start..start + page_size)
		.with_context(|| format!("sqlite v1 page {pgno} is outside the recovered database"))?;

	Ok(DirtyPage {
		pgno,
		bytes: bytes.to_vec(),
	})
}

struct MigrationState {
	head: Option<DBHead>,
	in_progress_pages: Option<u32>,
}

async fn load_migration_state(
	db: &universaldb::Database,
	recipient: &Recipient,
) -> Result<MigrationState> {
	let actor_id = recipient.actor_id;
	let database_id = actor_id.to_string();
	let bucket_id = BucketId::from_gas_id(recipient.namespace_id);
	db.txn("pegboard_actor_sqlite_get_migration_state", move |tx| {
		let database_id = database_id.clone();
		let bucket_id = bucket_id;
		async move {
			let key = if let Some(branch_id) = depot_branch::resolve_database_branch(
				&tx,
				bucket_id,
				&database_id,
				universaldb::utils::IsolationLevel::Snapshot,
			)
			.await?
			{
				depot_keys::branch_meta_head_key(branch_id)
			} else {
				depot_keys::meta_head_key(&database_id)
			};
			let head = tx
				.informal()
				.get(&key, universaldb::utils::IsolationLevel::Snapshot)
				.await?
				.map(|bytes| decode_db_head(bytes.as_ref()))
				.transpose()
				.context("decode sqlite db head")?;

			let marker_key = crate::keys::actor::SqliteMigrationKey::new(actor_id);
			let in_progress_pages = tx
				.with_subspace(crate::keys::subspace())
				.read_opt(
					&marker_key,
					universaldb::utils::IsolationLevel::Serializable,
				)
				.await?;

			Ok(MigrationState {
				head,
				in_progress_pages,
			})
		}
	})
	.await
}

async fn write_migration_marker(
	db: &universaldb::Database,
	actor_id: Id,
	total_pages: Option<u32>,
) -> Result<()> {
	db.txn("pegboard_actor_sqlite_write_migration_marker", move |tx| {
		let total_pages = total_pages;
		async move {
			let tx = tx.with_subspace(crate::keys::subspace());
			let key = crate::keys::actor::SqliteMigrationKey::new(actor_id);
			match total_pages {
				Some(total_pages) => tx.write(&key, total_pages)?,
				None => tx.delete(&key),
			}

			Ok(())
		}
	})
	.await
}

fn migration_error(actor_id: &str, phase: &'static str, err: anyhow::Error) -> anyhow::Error {
	metrics::SQLITE_MIGRATION_FAILURES_TOTAL
		.with_label_values(&[phase])
		.inc();
	tracing::error!(actor_id = %actor_id, phase, ?err, "v1→v2 migration failed");
	err
}

async fn read_v1_main(db: &universaldb::Database, recipient: &Recipient) -> Result<V1File> {
	// v1->v2 migration is best-effort: abandon sidecars and import the main
	// database file as-is.
	abandon_v1_sidecar_if_exists(db, recipient, FILE_TAG_JOURNAL, "journal").await?;
	abandon_v1_sidecar_if_exists(db, recipient, FILE_TAG_WAL, "wal").await?;
	abandon_v1_sidecar_if_exists(db, recipient, FILE_TAG_SHM, "shm").await?;

	read_v1_file(db, recipient, FILE_TAG_MAIN)
		.await?
		.context("sqlite v1 main file missing metadata")
}

async fn abandon_v1_sidecar_if_exists(
	db: &universaldb::Database,
	recipient: &Recipient,
	file_tag: u8,
	sidecar: &'static str,
) -> Result<()> {
	if v1_file_exists(db, recipient, file_tag).await? {
		metrics::SQLITE_MIGRATION_ABANDONED_SIDECAR_TOTAL
			.with_label_values(&[sidecar])
			.inc();
		tracing::warn!(
			actor_id = %recipient.actor_id,
			sidecar,
			"abandoning sqlite v1 sidecar during v1→v2 migration"
		);
	}

	Ok(())
}

async fn v1_file_exists(
	db: &universaldb::Database,
	recipient: &Recipient,
	file_tag: u8,
) -> Result<bool> {
	let (meta_keys, _, _) =
		crate::actor_kv::get(db, recipient, vec![v1_meta_key(file_tag).to_vec()]).await?;
	if !meta_keys.is_empty() {
		return Ok(true);
	}

	let (keys, _, _) = crate::actor_kv::list(
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
		crate::actor_kv::get(db, recipient, vec![meta_key.clone()]).await?;

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
	let (chunk_keys, chunk_values, _) = crate::actor_kv::list(
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

	Ok(Some(V1File { bytes }))
}

fn validate_v1_main(actor_id: &str, main: V1File) -> Result<RecoveredDb> {
	let bytes = main.bytes;
	if bytes.is_empty() {
		return Ok(RecoveredDb {
			bytes,
			total_pages: 0,
		});
	}

	ensure!(
		bytes.len() >= SQLITE_MAGIC.len() + 2,
		"sqlite v1 main file too small for actor {actor_id}"
	);
	ensure!(
		&bytes[..SQLITE_MAGIC.len()] == SQLITE_MAGIC,
		"sqlite v1 magic bytes mismatch for actor {actor_id}"
	);
	let raw_page_size = u16::from_be_bytes([bytes[16], bytes[17]]);
	let page_size = if raw_page_size == 1 {
		65_536_u32
	} else {
		u32::from(raw_page_size)
	};
	ensure!(
		(512..=65_536).contains(&page_size),
		"sqlite v1 page size {page_size} is outside the supported range for actor {actor_id}"
	);
	ensure!(
		page_size == SQLITE_PAGE_SIZE,
		"sqlite v1 page size {page_size} is not supported by sqlite v2 for actor {actor_id}"
	);
	ensure!(
		bytes.len() % page_size as usize == 0,
		"sqlite v1 database size {} is not page aligned to {} for actor {actor_id}",
		bytes.len(),
		page_size
	);

	Ok(RecoveredDb {
		total_pages: (bytes.len() / page_size as usize) as u32,
		bytes,
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

struct V1File {
	bytes: Vec<u8>,
}

struct RecoveredDb {
	bytes: Vec<u8>,
	total_pages: u32,
}
