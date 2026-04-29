//! Open handling for SQLite lifecycle setup and preload.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use anyhow::{Context, Result, bail, ensure};

use crate::engine::{OpenDb, SqliteEngine};
use crate::error::SqliteStorageError;
use crate::keys::{
	decode_delta_chunk_txid, delta_chunk_prefix, delta_prefix, meta_key, pidx_delta_prefix,
	preload_hints_key, shard_key, shard_prefix,
};
use crate::ltx::decode_ltx_v3;
use crate::optimization_flags::{SqliteOptimizationFlags, sqlite_optimization_flags};
use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
use crate::types::{
	DBHead, FetchedPage, PreloadHints, SQLITE_MAX_DELTA_BYTES, SqliteMeta, SqliteOrigin,
	decode_db_head, decode_preload_hints, new_db_head,
};
use crate::udb::{self, WriteOp};

pub const DEFAULT_PRELOAD_MAX_BYTES: usize = 1024 * 1024;

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnoRange {
	pub start: u32,
	pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenConfig {
	pub now_ms: i64,
	pub preload_pgnos: Vec<u32>,
	pub preload_ranges: Vec<PgnoRange>,
	pub max_total_bytes: usize,
	pub preload_hints: OpenPreloadHintConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenPreloadHintConfig {
	pub enabled: bool,
	pub hot_pages: bool,
	pub early_pages: bool,
	pub scan_ranges: bool,
}

impl OpenPreloadHintConfig {
	pub fn from_optimization_flags(flags: SqliteOptimizationFlags) -> Self {
		Self {
			enabled: flags.preload_hints_on_open,
			hot_pages: flags.preload_hint_hot_pages,
			early_pages: flags.preload_hint_early_pages,
			scan_ranges: flags.preload_hint_scan_ranges,
		}
	}
}

impl OpenConfig {
	pub fn new(now_ms: i64) -> Self {
		Self {
			now_ms,
			preload_pgnos: Vec::new(),
			preload_ranges: Vec::new(),
			max_total_bytes: DEFAULT_PRELOAD_MAX_BYTES,
			preload_hints: OpenPreloadHintConfig::from_optimization_flags(
				*sqlite_optimization_flags(),
			),
		}
	}
}

struct OpenPlaceholderGuard<'a> {
	open_dbs: &'a scc::HashMap<String, OpenDb>,
	actor_id: String,
	reservation_generation: u64,
	disarmed: bool,
}

impl<'a> Drop for OpenPlaceholderGuard<'a> {
	fn drop(&mut self) {
		if self.disarmed {
			return;
		}
		// Synchronous remove from `scc::HashMap` is safe because no dependent
		// state holds a lock here.
		self.open_dbs
			.remove_if_sync(&self.actor_id, |open_db| {
				open_db.generation == self.reservation_generation
			});
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenResult {
	pub generation: u64,
	pub meta: SqliteMeta,
	pub preloaded_pages: Vec<FetchedPage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareV1MigrationResult {
	pub meta: SqliteMeta,
}

impl SqliteEngine {
	pub async fn prepare_v1_migration(
		&self,
		actor_id: &str,
		now_ms: i64,
	) -> Result<PrepareV1MigrationResult> {
		self.reset_v1_migration(actor_id, now_ms, false)
			.await?
			.context("v1 migration reset unexpectedly returned no state")
	}

	pub async fn invalidate_v1_migration(&self, actor_id: &str, now_ms: i64) -> Result<bool> {
		Ok(self
			.reset_v1_migration(actor_id, now_ms, true)
			.await?
			.is_some())
	}

	async fn reset_v1_migration(
		&self,
		actor_id: &str,
		now_ms: i64,
		require_stage_in_progress: bool,
	) -> Result<Option<PrepareV1MigrationResult>> {
		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let head = udb::run_db_op(&self.db, self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			async move {
				let meta_storage_key = meta_key(&actor_id);
				if let Some(existing_meta) =
					udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key).await?
				{
					let existing_head = decode_db_head(&existing_meta)?;
					if !matches!(existing_head.origin, SqliteOrigin::MigrationFromV1InProgress) {
						// Actor has already moved past v1 migration (CreatedOnV2 or
						// MigratedFromV1). For invalidate_v1_migration this is a
						// no-op — there is nothing stale to clean up. For
						// prepare_v1_migration this is a bug because the caller
						// is trying to start a fresh v1 migration over an actor
						// already on v2.
						if require_stage_in_progress {
							return Ok(None);
						}
						bail!(SqliteStorageError::InvalidV1MigrationState);
					}
					let stage_in_progress =
						existing_head.next_txid > existing_head.head_txid.saturating_add(1);
					if require_stage_in_progress && !stage_in_progress {
						return Ok(None);
					}
				} else if require_stage_in_progress {
					return Ok(None);
				}

				udb::tx_delete_value_precise(&tx, &subspace, &meta_storage_key).await?;
				for prefix in [
					delta_prefix(&actor_id),
					pidx_delta_prefix(&actor_id),
					shard_prefix(&actor_id),
				] {
					for (key, _) in udb::tx_scan_prefix_values(&tx, &subspace, &prefix).await? {
						udb::tx_delete_value_precise(&tx, &subspace, &key).await?;
					}
				}
				udb::tx_delete_value_precise(&tx, &subspace, &preload_hints_key(&actor_id))
					.await?;

				let mut head = new_db_head(now_ms);
				head.origin = SqliteOrigin::MigrationFromV1InProgress;
				let (head, encoded_head) = encode_db_head_with_usage(&actor_id, &head, 0)?;
				udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;

				Ok(Some(head))
			}
		})
		.await?;

		self.page_indices.remove_async(&actor_id).await;
		self.pending_stages
			.retain_sync(|(pending_actor_id, _), _| pending_actor_id != &actor_id);

		Ok(head.map(|head| PrepareV1MigrationResult {
			meta: SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES)),
		}))
	}

	pub async fn open(&self, actor_id: &str, config: OpenConfig) -> Result<OpenResult> {
		let actor_id_string = actor_id.to_string();
		let takeover_min_generation = match self.open_dbs.entry_async(actor_id_string.clone()).await
		{
			scc::hash_map::Entry::Occupied(mut entry) => {
				let next_generation = entry.get().generation.max(1).saturating_add(1);
				entry.get_mut().generation = next_generation;
				Some(next_generation)
			}
			scc::hash_map::Entry::Vacant(entry) => {
				entry.insert_entry(OpenDb { generation: 0 });
				None
			}
		};
		let reservation_generation = takeover_min_generation.unwrap_or(0);

		// Drop guard removes the placeholder if the future is cancelled or
		// `open_inner` errors. Without this a dropped future would leave a
		// placeholder in `open_dbs` that permanently blocks re-opening the
		// actor on this process.
		let guard = OpenPlaceholderGuard {
			open_dbs: &self.open_dbs,
			actor_id: actor_id_string.clone(),
			reservation_generation,
			disarmed: false,
		};

		let result = async {
			if let Some(min_generation) = takeover_min_generation {
				self.bump_generation_for_takeover(&actor_id_string, config.now_ms, min_generation)
					.await?;
			}
			self.open_inner(actor_id, config).await
		}
		.await;
		// Disarm the guard so the placeholder is not removed before we either
		// promote it (Ok path) or remove it explicitly (Err path).
		let mut guard = guard;
		guard.disarmed = true;
		drop(guard);

		match result {
			Ok(result) => {
				self.open_dbs
					.update_async(actor_id, |_, open_db| {
						if open_db.generation <= result.generation {
							open_db.generation = result.generation;
						}
					})
					.await
					.context("sqlite open state missing after open")?;
				Ok(result)
			}
			Err(err) => {
				self.open_dbs
					.remove_if_async(actor_id, |open_db| {
						open_db.generation == reservation_generation
					})
					.await;
				Err(err)
			}
		}
	}

	async fn bump_generation_for_takeover(
		&self,
		actor_id: &str,
		now_ms: i64,
		min_generation: u64,
	) -> Result<()> {
		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();

		udb::run_db_op(&self.db, self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			async move {
				let meta_storage_key = meta_key(&actor_id);
				let meta_bytes =
					udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key).await?;
				let (mut head, usage_without_meta) = if let Some(meta_bytes) = meta_bytes.as_ref() {
					let head = decode_db_head(meta_bytes)?;
					let usage_without_meta = head.sqlite_storage_used.saturating_sub(
						tracked_storage_entry_size(&meta_storage_key, meta_bytes)
							.expect("meta key should count toward sqlite quota"),
					);
					(head, usage_without_meta)
				} else {
					(new_db_head(now_ms), 0)
				};

				head.generation = head.generation.saturating_add(1).max(min_generation);
				let (_, encoded_head) =
					encode_db_head_with_usage(&actor_id, &head, usage_without_meta)?;
				udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;

				Ok(())
			}
		})
		.await?;

		self.page_indices.remove_async(&actor_id).await;
		self.pending_stages
			.retain_sync(|(pending_actor_id, _), _| pending_actor_id != &actor_id);

		Ok(())
	}

	async fn open_inner(&self, actor_id: &str, config: OpenConfig) -> Result<OpenResult> {
		let start = Instant::now();
		let meta_bytes = udb::get_value(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?;
		let mut live_pidx = BTreeMap::new();
		let mut mutations = Vec::new();
		let mut should_schedule_compaction = false;
		let mut recovered_orphans = 0usize;
		let mut recovered_orphan_bytes = 0u64;
		let usage_without_meta = if let Some(meta_bytes) = meta_bytes.as_ref() {
			let head = decode_db_head(meta_bytes)?;
			head.sqlite_storage_used.saturating_sub(
				tracked_storage_entry_size(&meta_key(actor_id), meta_bytes)
					.expect("meta key should count toward sqlite quota"),
			)
		} else {
			0
		};

		let head = if let Some(meta_bytes) = meta_bytes.clone() {
			let head = decode_db_head(&meta_bytes)?;
			let recovery_plan = self
				.build_recovery_plan(actor_id, &head, &mut live_pidx)
				.await?;
			should_schedule_compaction = recovery_plan.live_delta_count >= 32;
			let tracked_deleted_bytes = recovery_plan.tracked_deleted_bytes;
			recovered_orphans = recovery_plan.orphan_count;
			recovered_orphan_bytes = tracked_deleted_bytes;
			mutations.extend(recovery_plan.mutations);
			let mut head = head;
			head.sqlite_storage_used = usage_without_meta.saturating_sub(tracked_deleted_bytes);
			head
		} else {
			new_db_head(config.now_ms)
		};

		let head_for_tx = head.clone();
		let actor_id_for_tx = actor_id.to_string();
		let open_mutations = mutations.clone();
		let subspace = self.subspace.clone();
		let head = udb::run_db_op(&self.db, self.op_counter.as_ref(), move |tx| {
			let open_mutations = open_mutations.clone();
			let subspace = subspace.clone();
			let actor_id = actor_id_for_tx.clone();
			let head = head_for_tx.clone();
			async move {
				let meta_storage_key = meta_key(&actor_id);
				let mut head = head;
				if let Some(current_meta_bytes) =
					udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key).await?
				{
					let current_head = decode_db_head(&current_meta_bytes)?;
					head.generation = head.generation.max(current_head.generation);
				}

				for op in &open_mutations {
					match op {
						WriteOp::Put(key, value) => {
							udb::tx_write_value(&tx, &subspace, key, value)?
						}
						WriteOp::Delete(key) => udb::tx_delete_value(&tx, &subspace, key),
					}
				}
				let (head, encoded_head) =
					encode_db_head_with_usage(&actor_id, &head, head.sqlite_storage_used)?;
				udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;

				tracing::debug!(actor_id = %actor_id, "opened sqlite db");
				Ok(head)
			}
		})
		.await?;
		if should_schedule_compaction {
			let _ = self.compaction_tx.send(actor_id.to_string());
		}
		self.metrics.add_recovery_orphans_cleaned(recovered_orphans);
		self.metrics
			.add_orphan_chunk_bytes_reclaimed(recovered_orphan_bytes);
		self.metrics.set_delta_count_from_head(&head);

		self.page_indices.remove_async(&actor_id.to_string()).await;

		let persisted_preload_hints = self.load_preload_hints(actor_id, &config).await?;
		let preloaded_pages = self
			.preload_pages(
				actor_id,
				&head,
				&live_pidx,
				&config,
				persisted_preload_hints.as_ref(),
			)
			.await?;
		let meta = SqliteMeta::from((head.clone(), SQLITE_MAX_DELTA_BYTES));
		self.metrics.observe_open(start.elapsed());

		Ok(OpenResult {
			generation: head.generation,
			meta,
			preloaded_pages,
		})
	}

	pub async fn close(&self, actor_id: &str, generation: u64) -> Result<()> {
		let actor_id = actor_id.to_string();
		self.ensure_open(&actor_id, generation, "close").await?;
		let removed = self
			.open_dbs
			.remove_if_async(&actor_id, |open_db| open_db.generation == generation)
			.await;
		ensure!(removed.is_some(), "sqlite db is not open for actor");

		self.page_indices.remove_async(&actor_id).await;
		self.pending_stages
			.retain_sync(|(pending_actor_id, _), _| pending_actor_id != &actor_id);

		Ok(())
	}

	pub async fn ensure_local_open(&self, actor_id: &str, generation: u64) -> Result<()> {
		let head = self.load_head(actor_id).await?;
		ensure!(
			head.generation == generation,
			SqliteStorageError::FenceMismatch {
				reason: format!(
					"ensure_local_open generation {} did not match current generation {}",
					generation, head.generation
				),
			},
		);

		match self.open_dbs.entry_async(actor_id.to_string()).await {
			scc::hash_map::Entry::Occupied(entry) => {
				ensure!(
					entry.get().generation == generation,
					SqliteStorageError::FenceMismatch {
						reason: format!(
							"ensure_local_open generation {} did not match open generation {}",
							generation,
							entry.get().generation
						),
					},
				);
			}
			scc::hash_map::Entry::Vacant(entry) => {
				entry.insert_entry(OpenDb { generation });
			}
		}

		Ok(())
	}

	// Unconditionally evict the actor's open-db / page-index / pending-stage caches without
	// generation fencing. Use only on shutdown paths where keeping a stale entry would block
	// future opens of the same actor on this process-wide engine.
	pub async fn force_close(&self, actor_id: &str) {
		let actor_id = actor_id.to_string();
		self.open_dbs.remove_async(&actor_id).await;
		self.page_indices.remove_async(&actor_id).await;
		self.pending_stages
			.retain_sync(|(pending_actor_id, _), _| pending_actor_id != &actor_id);
	}

	pub(crate) async fn ensure_open(
		&self,
		actor_id: &str,
		generation: u64,
		operation: &'static str,
	) -> Result<()> {
		let open_db_generation = self
			.open_dbs
			.read_async(actor_id, |_, open_db| open_db.generation)
			.await
			.ok_or(SqliteStorageError::DbNotOpen { operation })?;
		ensure!(
			open_db_generation == generation,
			SqliteStorageError::FenceMismatch {
				reason: format!(
					"{operation} generation {} did not match open generation {}",
					generation, open_db_generation
				),
			}
		);

		Ok(())
	}

	async fn build_recovery_plan(
		&self,
		actor_id: &str,
		head: &DBHead,
		live_pidx: &mut BTreeMap<u32, u64>,
	) -> Result<RecoveryPlan> {
		let delta_rows = udb::scan_prefix_values(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			delta_prefix(actor_id),
		)
		.await?;
		let pidx_rows = udb::scan_prefix_values(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			pidx_delta_prefix(actor_id),
		)
		.await?;
		let shard_rows = udb::scan_prefix_values(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			shard_prefix(actor_id),
		)
		.await?;

		let mut delta_rows_by_txid = BTreeMap::<u64, Vec<(Vec<u8>, Vec<u8>)>>::new();
		let mut mutations = Vec::new();
		let mut tracked_deleted_bytes = 0u64;

		for (key, value) in delta_rows {
			let txid = decode_delta_chunk_txid(actor_id, &key)?;
			delta_rows_by_txid
				.entry(txid)
				.or_default()
				.push((key, value));
		}

		let mut live_delta_txids = BTreeSet::new();
		for (key, value) in pidx_rows {
			let pgno = decode_pidx_pgno(actor_id, &key)?;
			let txid = decode_pidx_txid(&value)?;

			if pgno == 0
				|| pgno > head.db_size_pages
				|| txid > head.head_txid
				|| !delta_rows_by_txid.contains_key(&txid)
			{
				tracked_deleted_bytes += tracked_storage_entry_size(&key, &value)
					.expect("pidx key should count toward sqlite quota");
				mutations.push(WriteOp::delete(key));
			} else {
				live_pidx.insert(pgno, txid);
				live_delta_txids.insert(txid);
			}
		}

		for (txid, rows) in delta_rows_by_txid {
			if txid > head.head_txid || !live_delta_txids.contains(&txid) {
				for (key, value) in rows {
					tracked_deleted_bytes += tracked_storage_entry_size(&key, &value)
						.expect("delta key should count toward sqlite quota");
					mutations.push(WriteOp::delete(key));
				}
			}
		}

		for (key, value) in shard_rows {
			let shard_id = decode_shard_id(actor_id, &key)?;
			if shard_id.saturating_mul(head.shard_size) > head.db_size_pages {
				tracked_deleted_bytes += tracked_storage_entry_size(&key, &value)
					.expect("shard key should count toward sqlite quota");
				mutations.push(WriteOp::delete(key));
			}
		}
		let orphan_count = mutations.len();

		Ok(RecoveryPlan {
			mutations,
			live_delta_count: live_delta_txids.len(),
			orphan_count,
			tracked_deleted_bytes,
		})
	}

	async fn load_preload_hints(
		&self,
		actor_id: &str,
		config: &OpenConfig,
	) -> Result<Option<PreloadHints>> {
		if !config.preload_hints.enabled {
			return Ok(None);
		}

		let Some(bytes) = udb::get_value(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			preload_hints_key(actor_id),
		)
		.await?
		else {
			return Ok(None);
		};

		let hints = decode_preload_hints(&bytes)?;
		tracing::debug!(
			actor_id,
			hint_pages = hints.pgnos.len(),
			hint_ranges = hints.ranges.len(),
			hot_pages_enabled = config.preload_hints.hot_pages,
			early_pages_enabled = config.preload_hints.early_pages,
			scan_ranges_enabled = config.preload_hints.scan_ranges,
			max_total_bytes = config.max_total_bytes,
			"loaded sqlite preload hints"
		);
		Ok(Some(hints))
	}

	async fn preload_pages(
		&self,
		actor_id: &str,
		head: &DBHead,
		live_pidx: &BTreeMap<u32, u64>,
		config: &OpenConfig,
		persisted_hints: Option<&PreloadHints>,
	) -> Result<Vec<FetchedPage>> {
		let requested = collect_preload_pgnos(config, persisted_hints);
		let mut sources = BTreeMap::new();

		for pgno in &requested {
			if *pgno == 0 || *pgno > head.db_size_pages {
				continue;
			}

			let key = if let Some(txid) = live_pidx.get(pgno) {
				delta_chunk_prefix(actor_id, *txid)
			} else {
				shard_key(actor_id, *pgno / head.shard_size)
			};
			sources.insert(key, None);
		}

		if !sources.is_empty() {
			let keys = sources.keys().cloned().collect::<Vec<_>>();
			for key in keys {
				let value = if key.starts_with(&delta_prefix(actor_id)) {
					load_delta_blob(
						&self.db,
						&self.subspace,
						self.op_counter.as_ref(),
						key.as_slice(),
					)
					.await?
				} else {
					udb::get_value(
						&self.db,
						&self.subspace,
						self.op_counter.as_ref(),
						key.clone(),
					)
					.await?
				};
				sources.insert(key, value);
			}
		}

		let mut decoded_pages = BTreeMap::new();
		let mut total_bytes = 0usize;
		let mut preloaded_pages = Vec::with_capacity(requested.len());

		for pgno in requested {
			if pgno == 0 || pgno > head.db_size_pages {
				preloaded_pages.push(FetchedPage { pgno, bytes: None });
				continue;
			}

			let source_key = if let Some(txid) = live_pidx.get(&pgno) {
				delta_chunk_prefix(actor_id, *txid)
			} else {
				shard_key(actor_id, pgno / head.shard_size)
			};

			let page_bytes = match sources.get(&source_key).cloned().flatten() {
				Some(blob) => {
					let cached = decoded_pages.contains_key(&source_key);
					if !cached {
						let decoded_ltx = decode_ltx_v3(&blob)
							.with_context(|| format!("decode preload blob for page {pgno}"))?;
						decoded_pages.insert(source_key.clone(), decoded_ltx.pages);
					}

					decoded_pages.get(&source_key).and_then(|pages| {
						pages
							.iter()
							.find(|page| page.pgno == pgno)
							.map(|page| page.bytes.clone())
					})
				}
				None => None,
			};

			match page_bytes {
				Some(bytes) if pgno == 1 || total_bytes + bytes.len() <= config.max_total_bytes => {
					total_bytes += bytes.len();
					preloaded_pages.push(FetchedPage {
						pgno,
						bytes: Some(bytes),
					});
				}
				Some(_) | None => {
					preloaded_pages.push(FetchedPage { pgno, bytes: None });
				}
			}
		}

		Ok(preloaded_pages)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryPlan {
	mutations: Vec<WriteOp>,
	live_delta_count: usize,
	orphan_count: usize,
	tracked_deleted_bytes: u64,
}

fn collect_preload_pgnos(
	config: &OpenConfig,
	persisted_hints: Option<&PreloadHints>,
) -> Vec<u32> {
	let mut requested = BTreeSet::from([1]);
	for pgno in &config.preload_pgnos {
		if *pgno > 0 {
			requested.insert(*pgno);
		}
	}

	for range in &config.preload_ranges {
		for pgno in range.start..range.end {
			if pgno > 0 {
				requested.insert(pgno);
			}
		}
	}

	if let Some(hints) = persisted_hints {
		if config.preload_hints.hot_pages || config.preload_hints.early_pages {
			for pgno in &hints.pgnos {
				if *pgno > 0 {
					requested.insert(*pgno);
				}
			}
		}

		if config.preload_hints.scan_ranges {
			for range in &hints.ranges {
				let end = range.start_pgno.saturating_add(range.page_count);
				for pgno in range.start_pgno..end {
					if pgno > 0 {
						requested.insert(pgno);
					}
				}
			}
		}
	}

	requested.into_iter().collect()
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = pidx_delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == PIDX_TXID_BYTES,
		"pidx value had {} bytes, expected {}",
		value.len(),
		PIDX_TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}

fn decode_shard_id(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = shard_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"shard key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"shard key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("shard key suffix should decode as u32")?,
	))
}

async fn load_delta_blob(
	db: &universaldb::Database,
	subspace: &universaldb::Subspace,
	op_counter: &std::sync::atomic::AtomicUsize,
	delta_prefix: &[u8],
) -> Result<Option<Vec<u8>>> {
	let delta_chunks =
		udb::scan_prefix_values(db, subspace, op_counter, delta_prefix.to_vec()).await?;
	if delta_chunks.is_empty() {
		return Ok(None);
	}

	let mut delta_blob = Vec::new();
	for (_, chunk) in delta_chunks {
		delta_blob.extend_from_slice(&chunk);
	}

	Ok(Some(delta_blob))
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use tokio::sync::mpsc::error::TryRecvError;

	use rivet_metrics::REGISTRY;
	use rivet_metrics::prometheus::{Encoder, TextEncoder};

	use super::{OpenConfig, PgnoRange};
	use crate::commit::CommitStageRequest;
	use crate::engine::SqliteEngine;
	use crate::keys::{delta_chunk_key, meta_key, pidx_delta_key, preload_hints_key, shard_key};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
	use crate::test_utils::{
		checkpoint_test_db, read_value, reopen_test_db, scan_prefix_values, test_db,
		test_db_with_path,
	};

	fn registry_text() -> String {
		let mut buffer = Vec::new();
		TextEncoder::new()
			.encode(&REGISTRY.gather(), &mut buffer)
			.expect("metrics encode");
		String::from_utf8(buffer).expect("metrics utf8")
	}
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, PreloadHintRange, PreloadHints,
		SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_MAX_DELTA_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin, decode_db_head,
		encode_db_head, encode_preload_hints,
	};
	use crate::udb::{WriteOp, apply_write_ops, physical_chunk_key, raw_key_exists};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 1,
			head_txid: 3,
			next_txid: 4,
			materialized_txid: 0,
			db_size_pages: 4,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms: 123,
			sqlite_storage_used: 0,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
			origin: SqliteOrigin::CreatedOnV2,
		}
	}

	fn page(fill: u8) -> Vec<u8> {
		vec![fill; SQLITE_PAGE_SIZE as usize]
	}

	fn delta_blob_key(actor_id: &str, txid: u64) -> Vec<u8> {
		delta_chunk_key(actor_id, txid, 0)
	}

	fn encoded_blob(txid: u64, pgno: u32, fill: u8) -> Vec<u8> {
		encode_ltx_v3(
			LtxHeader::delta(txid, pgno, 999),
			&[DirtyPage {
				pgno,
				bytes: page(fill),
			}],
		)
		.expect("encode test ltx blob")
	}

	async fn actual_tracked_usage(engine: &SqliteEngine) -> Result<u64> {
		Ok(scan_prefix_values(engine, vec![0x02])
			.await?
			.into_iter()
			.filter_map(|(key, value)| tracked_storage_entry_size(&key, &value))
			.sum())
	}

	async fn rewrite_meta_with_actual_usage(engine: &SqliteEngine, actor_id: &str) -> Result<()> {
		let meta_key = meta_key(actor_id);
		let meta_bytes = read_value(engine, meta_key.clone())
			.await?
			.expect("meta should exist before rewrite");
		let head = decode_db_head(&meta_bytes)?;
		let usage_without_meta = actual_tracked_usage(engine).await?.saturating_sub(
			tracked_storage_entry_size(&meta_key, &meta_bytes)
				.expect("meta key should count toward sqlite quota"),
		);
		let (_, rewritten_meta) = encode_db_head_with_usage(actor_id, &head, usage_without_meta)?;
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key, rewritten_meta)],
		)
		.await?;
		Ok(())
	}

	#[tokio::test]
	async fn open_on_empty_store_creates_meta_and_page_one_placeholder() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let result = engine.open(TEST_ACTOR, OpenConfig::new(777)).await?;

		assert_eq!(result.generation, 1);
		assert_eq!(result.meta.generation, 1);
		assert_eq!(result.meta.head_txid, 0);
		assert_eq!(result.meta.max_delta_bytes, SQLITE_MAX_DELTA_BYTES);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: None,
			}]
		);
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		let stored_meta = read_value(&engine, meta_key(TEST_ACTOR))
			.await?
			.expect("meta should exist");
		let head = decode_db_head(&stored_meta)?;
		assert_eq!(head.generation, 1);
		assert_eq!(head.creation_ts_ms, 777);

		Ok(())
	}

	#[tokio::test]
	async fn prepare_v1_migration_wipes_actor_rows_and_chunk_subkeys() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let large_orphan = vec![0x5a; 150_000];
		let unrelated_key = meta_key("other-actor");
		let orphan_key = delta_blob_key(TEST_ACTOR, 99);
		let orphan_chunk_0 = physical_chunk_key(&engine.subspace, &orphan_key, 0);
		let orphan_chunk_14 = physical_chunk_key(&engine.subspace, &orphan_key, 14);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(orphan_key.clone(), large_orphan),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 99_u64.to_be_bytes().to_vec()),
				WriteOp::put(unrelated_key.clone(), vec![0x42]),
			],
		)
		.await?;

		assert!(
			raw_key_exists(
				&engine.db,
				engine.op_counter.as_ref(),
				orphan_chunk_0.clone(),
			)
			.await?,
			"chunked orphan should create physical chunk rows"
		);
		assert!(
			raw_key_exists(
				&engine.db,
				engine.op_counter.as_ref(),
				orphan_chunk_14.clone(),
			)
			.await?,
			"chunked orphan should create the tail chunk row too"
		);

		let prepared = engine.prepare_v1_migration(TEST_ACTOR, 4_242).await?;
		assert_eq!(prepared.meta.origin, SqliteOrigin::MigrationFromV1InProgress);

		assert!(read_value(&engine, orphan_key.clone()).await?.is_none());
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		let stored_meta = read_value(&engine, meta_key(TEST_ACTOR))
			.await?
			.expect("meta should be recreated");
		let head = decode_db_head(&stored_meta)?;
		assert_eq!(head.origin, SqliteOrigin::MigrationFromV1InProgress);
		assert_eq!(head.creation_ts_ms, 4_242);
		assert!(
			!raw_key_exists(&engine.db, engine.op_counter.as_ref(), orphan_chunk_0,).await?,
			"orphaned chunk row 0 should be wiped"
		);
		assert!(
			!raw_key_exists(&engine.db, engine.op_counter.as_ref(), orphan_chunk_14,).await?,
			"orphaned chunk subkeys should be wiped too"
		);

		assert_eq!(
			read_value(&engine, unrelated_key.clone()).await?,
			Some(vec![0x42]),
			"cleanup should stay inside the actor prefix"
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_on_existing_meta_keeps_generation_and_preloads_page_one() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(1, 1, 0x2a)),
			],
		)
		.await?;
		let result = engine.open(TEST_ACTOR, OpenConfig::new(888)).await?;

		assert_eq!(result.generation, 1);
		assert_eq!(result.meta.generation, 1);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x2a)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn preload_returns_requested_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 70;
		head.head_txid = 7;
		head.next_txid = 8;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 7),
					encode_ltx_v3(
						LtxHeader::delta(7, 70, 999),
						&[
							DirtyPage {
								pgno: 1,
								bytes: page(0x11),
							},
							DirtyPage {
								pgno: 2,
								bytes: page(0x22),
							},
						],
					)?,
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encode_ltx_v3(
						LtxHeader::delta(6, 70, 888),
						&[DirtyPage {
							pgno: 65,
							bytes: page(0x65),
						}],
					)?,
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 7_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 7_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		let mut config = OpenConfig::new(1_234);
		config.preload_pgnos = vec![65];
		config.preload_ranges.push(PgnoRange { start: 2, end: 3 });

		let result = engine.open(TEST_ACTOR, config).await?;
		assert_eq!(
			result.preloaded_pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x22)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_uses_persisted_preload_hints_by_default() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 70;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					shard_key(TEST_ACTOR, 0),
					encode_ltx_v3(
						LtxHeader::delta(1, 70, 999),
						&[
							DirtyPage {
								pgno: 1,
								bytes: page(0x11),
							},
							DirtyPage {
								pgno: 2,
								bytes: page(0x22),
							},
						],
					)?,
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encode_ltx_v3(
						LtxHeader::delta(1, 70, 999),
						&[
							DirtyPage {
								pgno: 65,
								bytes: page(0x65),
							},
							DirtyPage {
								pgno: 66,
								bytes: page(0x66),
							},
						],
					)?,
				),
				WriteOp::put(
					preload_hints_key(TEST_ACTOR),
					encode_preload_hints(&PreloadHints {
						pgnos: vec![2],
						ranges: vec![PreloadHintRange {
							start_pgno: 65,
							page_count: 2,
						}],
					})?,
				),
			],
		)
		.await?;

		let result = engine.open(TEST_ACTOR, OpenConfig::new(1_234)).await?;

		assert_eq!(
			result.preloaded_pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x22)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
				FetchedPage {
					pgno: 66,
					bytes: Some(page(0x66)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_can_disable_persisted_preload_hints() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 70;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(1, 1, 0x11)),
				WriteOp::put(
					preload_hints_key(TEST_ACTOR),
					encode_preload_hints(&PreloadHints {
						pgnos: vec![2],
						ranges: vec![PreloadHintRange {
							start_pgno: 65,
							page_count: 2,
						}],
					})?,
				),
			],
		)
		.await?;

		let mut config = OpenConfig::new(1_234);
		config.preload_hints.enabled = false;
		let result = engine.open(TEST_ACTOR, config).await?;

		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x11)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_can_disable_persisted_preload_scan_ranges() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 70;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					shard_key(TEST_ACTOR, 0),
					encode_ltx_v3(
						LtxHeader::delta(1, 70, 999),
						&[
							DirtyPage {
								pgno: 1,
								bytes: page(0x11),
							},
							DirtyPage {
								pgno: 2,
								bytes: page(0x22),
							},
						],
					)?,
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 1),
					encoded_blob(1, 65, 0x65),
				),
				WriteOp::put(
					preload_hints_key(TEST_ACTOR),
					encode_preload_hints(&PreloadHints {
						pgnos: vec![2],
						ranges: vec![PreloadHintRange {
							start_pgno: 65,
							page_count: 1,
						}],
					})?,
				),
			],
		)
		.await?;

		let mut config = OpenConfig::new(1_234);
		config.preload_hints.scan_ranges = false;
		let result = engine.open(TEST_ACTOR, config).await?;

		assert_eq!(
			result.preloaded_pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x22)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_keeps_generation() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(shard_key(TEST_ACTOR, 0), encoded_blob(1, 1, 0x2a)),
			],
		)
		.await?;

		let result = engine.open(TEST_ACTOR, OpenConfig::new(888)).await?;

		assert_eq!(result.generation, 1);
		assert_eq!(result.meta.generation, 1);

		Ok(())
	}

	#[tokio::test]
	async fn open_cleans_orphans_and_stale_pidx_entries() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&seeded_head())?),
				WriteOp::put(delta_blob_key(TEST_ACTOR, 2), encoded_blob(2, 1, 0x11)),
				WriteOp::put(delta_blob_key(TEST_ACTOR, 5), encoded_blob(5, 2, 0x55)),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 99_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		let result = engine.open(TEST_ACTOR, OpenConfig::new(999)).await?;

		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x11)),
			}]
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 2))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 5))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 3))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_cleans_above_eof_pidx_delta_and_shard_rows() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut head = seeded_head();
		head.db_size_pages = 2;
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(delta_blob_key(TEST_ACTOR, 1), encoded_blob(1, 1, 0x11)),
				WriteOp::put(delta_blob_key(TEST_ACTOR, 2), encoded_blob(2, 70, 0x70)),
				WriteOp::put(shard_key(TEST_ACTOR, 1), encoded_blob(3, 70, 0x71)),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 70), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		rewrite_meta_with_actual_usage(&engine, TEST_ACTOR).await?;
		let before_usage = actual_tracked_usage(&engine).await?;

		let result = engine.open(TEST_ACTOR, OpenConfig::new(999)).await?;

		assert_eq!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 1)).await?,
			Some(1_u64.to_be_bytes().to_vec())
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 70))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, shard_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		let after_usage = actual_tracked_usage(&engine).await?;
		assert!(after_usage < before_usage);
		assert_eq!(result.meta.sqlite_storage_used, after_usage);

		Ok(())
	}

	#[tokio::test]
	async fn open_cleans_orphan_deltas() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&seeded_head())?),
				WriteOp::put(delta_blob_key(TEST_ACTOR, 2), encoded_blob(2, 1, 0x11)),
				WriteOp::put(delta_blob_key(TEST_ACTOR, 5), encoded_blob(5, 2, 0x55)),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		engine.open(TEST_ACTOR, OpenConfig::new(999)).await?;

		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 2))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 5))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_cleans_orphan_staged_delta_chunks() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&seeded_head())?),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 42, 0), vec![1, 2, 3]),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 42, 1), vec![4, 5, 6]),
			],
		)
		.await?;

		engine.open(TEST_ACTOR, OpenConfig::new(999)).await?;

		assert!(
			read_value(&engine, delta_chunk_key(TEST_ACTOR, 42, 0))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_chunk_key(TEST_ACTOR, 42, 1))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_cleans_multiple_aborted_stages() -> Result<()> {
		// Multiple partial commit_stage blobs (N>1 distinct orphan txids beyond head_txid)
		// should all be deleted in a single open pass along with any dangling PIDX entries.
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let head = DBHead {
			head_txid: 5,
			next_txid: 9,
			db_size_pages: 1,
			..seeded_head()
		};
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				// Three orphan staged txids (> head_txid).
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 6, 0), vec![0; 256]),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 6, 1), vec![0; 256]),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 7, 0), vec![0; 512]),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 8, 0), vec![0; 1024]),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 8, 1), vec![0; 1024]),
				WriteOp::put(delta_chunk_key(TEST_ACTOR, 8, 2), vec![0; 1024]),
				// Dangling PIDX pointing at an orphan txid.
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 10), 8_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		engine.open(TEST_ACTOR, OpenConfig::new(1_111)).await?;

		for (txid, chunk_idx) in [(6, 0), (6, 1), (7, 0), (8, 0), (8, 1), (8, 2)] {
			assert!(
				read_value(&engine, delta_chunk_key(TEST_ACTOR, txid, chunk_idx))
					.await?
					.is_none(),
				"chunk {txid}/{chunk_idx} should be reclaimed",
			);
		}
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 10))
				.await?
				.is_none(),
			"dangling PIDX should be reclaimed",
		);
		let metrics_output = registry_text();
		assert!(
			metrics_output.contains("sqlite_v2_recovery_orphans_cleaned_total"),
			"recovery orphan count metric should be emitted",
		);
		assert!(
			metrics_output.contains("sqlite_orphan_chunk_bytes_reclaimed_total"),
			"orphan bytes metric should be emitted",
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_recovers_from_checkpointed_mid_commit_stage_state() -> Result<()> {
		let (db, subspace, _db_path) = test_db_with_path().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db.clone(), subspace.clone());
		let head = DBHead {
			head_txid: 0,
			next_txid: 1,
			db_size_pages: 0,
			..seeded_head()
		};
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				encode_db_head(&head)?,
			)],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		let stage = engine
			.commit_stage_begin(
				TEST_ACTOR,
				crate::commit::CommitStageBeginRequest {
					generation: head.generation,
				},
			)
			.await?;
		engine
			.commit_stage(
				TEST_ACTOR,
				CommitStageRequest {
					generation: head.generation,
					txid: stage.txid,
					chunk_idx: 0,
					bytes: encode_ltx_v3(
						LtxHeader::delta(stage.txid, 1, 999),
						&[DirtyPage {
							pgno: 1,
							bytes: page(0x44),
						}],
					)?,
					is_last: true,
				},
			)
			.await?;
		let checkpoint_path = checkpoint_test_db(&engine.db)?;
		drop(engine);
		drop(db);

		let reopened_db = reopen_test_db(&checkpoint_path).await?;
		let (recovered_engine, _compaction_rx) = SqliteEngine::new(reopened_db, subspace);
		let result = recovered_engine
			.open(TEST_ACTOR, OpenConfig::new(2_222))
			.await?;
		let stored_head = decode_db_head(
			&read_value(&recovered_engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should still exist after recovery"),
		)?;

		assert_eq!(result.generation, head.generation);
		assert_eq!(result.meta.head_txid, 0);
		assert_eq!(stored_head.head_txid, 0);
		assert_eq!(stored_head.next_txid, 2);
		assert_eq!(
			result.preloaded_pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: None,
			}]
		);
		assert!(
			read_value(&recovered_engine, delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn open_schedules_compaction_when_delta_threshold_is_met() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 32;
		head.next_txid = 33;
		head.db_size_pages = 32;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let mut mutations = vec![WriteOp::put(
			meta_key(TEST_ACTOR),
			encode_db_head(&head)?,
		)];
		for txid in 1..=32_u64 {
			mutations.push(WriteOp::put(
				delta_blob_key(TEST_ACTOR, txid),
				encoded_blob(txid, txid as u32, txid as u8),
			));
			mutations.push(WriteOp::put(
				pidx_delta_key(TEST_ACTOR, txid as u32),
				txid.to_be_bytes().to_vec(),
			));
		}
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			mutations,
		)
		.await?;
		let mut config = OpenConfig::new(1111);
		config.preload_ranges.push(PgnoRange { start: 2, end: 4 });

		let result = engine.open(TEST_ACTOR, config).await?;

		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
		assert_eq!(
			result.preloaded_pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(1)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(2)),
				},
				FetchedPage {
					pgno: 3,
					bytes: Some(page(3)),
				},
			]
		);

		Ok(())
	}
}
