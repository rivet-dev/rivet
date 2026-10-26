//! Fixed SlateDB snapshots and filesystem sequence bounds with expiring, grant-bound handles.

use super::{
	filter::Catalog,
	store::{FilesystemStore, LockOperation},
};
use bytes::Bytes;
use depot_fs_core::{
	FsBaseLink, FsVolumeRecord, FsVolumeStatus,
	keys::slatedb as keys,
	store::{Row, RowKey, RowRange},
};
use depot_types::VolumeLogKey;
use parking_lot::Mutex;
use rivet_data::generated::shared_depot_types_v1::VolumeLogPosition;
use rivet_env::{Clock, MutationGuard};
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{BinaryComponent, DescendingU64},
};
use slatedb::{DbSnapshot, config::ScanOptions};
use std::{
	ops::Bound,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("filesystem read session expired")]
	TooOld,
	#[error("filesystem apply is behind requested position")]
	Lagging,
	#[error("filesystem read authority revoked")]
	Fenced,
	#[error("filesystem read exceeds its bound")]
	TooLarge,
	#[error("filesystem read session capacity exhausted")]
	Overloaded,
	#[error("filesystem volume is unavailable")]
	NotFound,
	#[error("inherited filesystem data is unavailable")]
	BaseUnavailable,
	#[error("corrupt filesystem row or read cursor")]
	Corrupt,
	#[error(transparent)]
	Storage(#[from] anyhow::Error),
}
impl From<slatedb::Error> for Error {
	fn from(error: slatedb::Error) -> Self {
		Self::Storage(error.into())
	}
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct SessionConfig {
	pub max_sessions: usize,
	pub max_read_session_ms: u64,
	/// How long a session stays registered after its last read; see
	/// `depot_fs_core::wire::DEFAULT_READ_SESSION_IDLE_MS`.
	pub read_session_idle_ms: u64,
	/// The idle timeout of a session opened at the latest position, such as one an HTTP
	/// directory cursor resumes; see `depot_fs_core::wire::DEFAULT_LATEST_SESSION_IDLE_MS`.
	pub latest_session_idle_ms: u64,
	pub max_rows: usize,
	pub max_read_bytes: usize,
	pub scan: ScanOptions,
	/// How chunk runs of at least `min_chunks` are scanned; see `BulkRunConfig`.
	pub bulk: BulkRunConfig,
}

/// Scan settings for large chunk runs, such as a table scan's read-ahead window, so bulk
/// transfers can stay out of the shared block cache and fetch SST blocks in parallel while
/// point reads keep the default scan. `min_chunks` of 0 treats every run as a normal one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BulkRunConfig {
	pub min_chunks: usize,
	/// Whether blocks a bulk run fetches are inserted into the block cache.
	pub cache_blocks: bool,
	/// Concurrent SST block fetches inside one bulk run's scan.
	pub fetch_tasks: usize,
}

impl Default for BulkRunConfig {
	fn default() -> Self {
		Self {
			min_chunks: 0,
			cache_blocks: true,
			fetch_tasks: 1,
		}
	}
}
impl Default for SessionConfig {
	fn default() -> Self {
		Self {
			max_sessions: depot_fs_core::wire::DEFAULT_MAX_SESSIONS,
			max_read_session_ms: depot_fs_core::wire::DEFAULT_READ_SESSION_MS,
			read_session_idle_ms: depot_fs_core::wire::DEFAULT_READ_SESSION_IDLE_MS,
			latest_session_idle_ms: depot_fs_core::wire::DEFAULT_LATEST_SESSION_IDLE_MS,
			max_rows: depot_fs_core::wire::DEFAULT_MAX_READ_ROWS,
			max_read_bytes: depot_fs_core::wire::DEFAULT_MAX_READ_BYTES,
			scan: ScanOptions::default()
				.with_cache_blocks(true)
				.with_read_ahead_bytes(4096),
			bulk: BulkRunConfig::default(),
		}
	}
}
struct Pin {
	catalog: Catalog,
	scope: keys::Scope,
	id: Uuid,
}
impl Drop for Pin {
	fn drop(&mut self) {
		if let Some(retention) = self.catalog.lock().get_mut(&self.scope) {
			retention.sessions.remove(&self.id);
		}
	}
}
struct Snapshot {
	db: Arc<DbSnapshot>,
	_pin: Option<Pin>,
	_slot: Option<tokio::sync::OwnedSemaphorePermit>,
	/// This session's count in the read session gauge, released with its slot.
	_usage: Option<rivet_metrics::IntGaugeGuard>,
}
/// A live read session as the session table sees it.
pub(crate) trait LiveSession {
	/// When the session ends unless a read extends it. It never moves earlier.
	fn deadline_boot_ms(&self) -> u64;
	fn revoke(&self);
}

impl LiveSession for Arc<ReadSession> {
	fn deadline_boot_ms(&self) -> u64 {
		ReadSession::deadline_boot_ms(self)
	}

	fn revoke(&self) {
		ReadSession::revoke(self)
	}
}

/// Live read sessions by id, with their deadlines in an indexed queue so expiry visits only
/// the sessions that expired.
pub(crate) struct SessionTable<S = Arc<ReadSession>> {
	by_id: std::collections::BTreeMap<Uuid, S>,
	expiry: rivet_state_machine::IndexedDeadlineQueue<Uuid>,
}

impl<S> Default for SessionTable<S> {
	fn default() -> Self {
		Self {
			by_id: Default::default(),
			expiry: Default::default(),
		}
	}
}

impl<S: LiveSession> SessionTable<S> {
	pub(crate) fn len(&self) -> usize {
		self.by_id.len()
	}

	pub(crate) fn get(&self, id: &Uuid) -> Option<&S> {
		self.by_id.get(id)
	}

	pub(crate) fn values(&self) -> impl Iterator<Item = &S> {
		self.by_id.values()
	}

	pub(crate) fn insert(&mut self, id: Uuid, session: S) {
		self.expiry.schedule(id, session.deadline_boot_ms());
		if let Some(old) = self.by_id.insert(id, session) {
			old.revoke();
		}
	}

	pub(crate) fn remove(&mut self, id: &Uuid) -> Option<S> {
		self.expiry.cancel(id);
		self.by_id.remove(id)
	}

	/// Revokes and drops every session whose deadline is at or before `now_boot_ms`, and
	/// returns how many it visited: only the ones whose queued deadline passed. A read extends
	/// a session's deadline without touching the queue, so a visited session that was read
	/// since is queued again at its later deadline instead of revoked.
	pub(crate) fn expire(&mut self, now_boot_ms: u64) -> usize {
		let due = self.expiry.pop_due(now_boot_ms, usize::MAX);
		for (_, id) in &due {
			let Some(deadline) = self.by_id.get(id).map(LiveSession::deadline_boot_ms) else {
				continue;
			};
			if deadline > now_boot_ms {
				self.expiry.schedule(*id, deadline);
			} else if let Some(session) = self.by_id.remove(id) {
				session.revoke();
			}
		}
		due.len()
	}

	/// Revokes and drops every session.
	pub(crate) fn revoke_all(&mut self) {
		for (_, session) in std::mem::take(&mut self.by_id) {
			session.revoke();
		}
		self.expiry = Default::default();
	}
}



pub struct ReadSession {
	pub id: Uuid,
	pub scope: keys::Scope,
	/// History proof at session creation; logical reads use as_of, never a later seq.
	pub applied: VolumeLogPosition,
	pub as_of: u64,
	/// The volume's cold position when the session opened. A session at or below it reads
	/// only SSTs; above it, its snapshot also holds applied rows in the memtable.
	pub cold_volume_seq: u64,
	/// Chain captured with this snapshot; materialization never rewrites an existing session.
	pub base_chain: Vec<FsBaseLink>,
	pub(super) base_reader: Option<Arc<dyn super::base::BaseChunkReader>>,
	/// The session's lifetime, fixed at open.
	pub expires_boot_ms: u64,
	/// When the session ends if nothing reads it before then: its last read plus the idle
	/// timeout, never past `expires_boot_ms`.
	idle_deadline_boot_ms: AtomicU64,
	/// Whether the session was opened at the latest position, which uses the latest-session
	/// idle timeout.
	latest: std::sync::atomic::AtomicBool,
	snapshot: Mutex<Option<Snapshot>>,
	clock: Arc<dyn Clock>,
	guard: Arc<dyn MutationGuard>,
	config: SessionConfig,
}
impl ReadSession {
	pub(crate) fn internal(
		db: Arc<DbSnapshot>,
		scope: keys::Scope,
		applied: VolumeLogPosition,
		clock: Arc<dyn Clock>,
		guard: Arc<dyn MutationGuard>,
		config: SessionConfig,
	) -> Self {
		Self {
			id: Uuid::nil(),
			scope,
			as_of: applied.volume_seq,
			// Internal sessions never answer client reads, so no read metric uses this.
			cold_volume_seq: applied.volume_seq,
			base_chain: Vec::new(),
			base_reader: None,
			applied,
			expires_boot_ms: u64::MAX,
			idle_deadline_boot_ms: AtomicU64::new(u64::MAX),
			latest: std::sync::atomic::AtomicBool::new(false),
			snapshot: Mutex::new(Some(Snapshot {
				db,
				_pin: None,
				_slot: None,
				_usage: None,
			})),
			clock,
			guard,
			config,
		}
	}
	pub fn revoke(&self) {
		self.snapshot.lock().take();
	}
	/// When the session ends: its idle deadline, which is never past its lifetime.
	pub(crate) fn deadline_boot_ms(&self) -> u64 {
		self.idle_deadline_boot_ms.load(Ordering::Acquire)
	}
	/// Gives a session opened at the latest position the longer latest-session idle timeout.
	pub(crate) fn idle_as_latest(&self, now_boot_ms: u64) {
		let deadline = now_boot_ms
			.saturating_add(self.config.latest_session_idle_ms)
			.min(self.expires_boot_ms);
		self.idle_deadline_boot_ms
			.fetch_max(deadline, Ordering::AcqRel);
		self.latest.store(true, Ordering::Release);
	}
	/// Extends the session's idle deadline for a read at `now_boot_ms`, up to its lifetime.
	fn touch(&self, now_boot_ms: u64) {
		let idle_ms = if self.latest.load(Ordering::Acquire) {
			self.config.latest_session_idle_ms
		} else {
			self.config.read_session_idle_ms
		};
		let deadline = now_boot_ms
			.saturating_add(idle_ms)
			.min(self.expires_boot_ms);
		self.idle_deadline_boot_ms
			.fetch_max(deadline, Ordering::AcqRel);
	}
	pub(super) fn check(&self) -> Result<()> {
		if !self.guard.may_send() {
			self.revoke();
			return Err(Error::Fenced);
		}
		if self.clock.observe().map_err(|_| Error::Fenced)?.boot_ms >= self.deadline_boot_ms() {
			self.revoke();
			return Err(Error::TooOld);
		}
		if self.snapshot.lock().is_none() {
			return Err(Error::TooOld);
		}
		Ok(())
	}
	fn snapshot(&self) -> Result<Arc<DbSnapshot>> {
		self.check()?;
		self.snapshot
			.lock()
			.as_ref()
			.map(|s| s.db.clone())
			.ok_or(Error::TooOld)
	}
	fn options(&self, count: usize) -> ScanOptions {
		self.config
			.scan
			.clone()
			.with_cache_blocks(true)
			.with_read_ahead_bytes(
				count
					.saturating_mul(4096)
					.min(self.config.max_read_bytes)
					.max(1),
			)
	}
	/// Preserves Deleted versus absent so fork chunk lookup can stop at an explicit mask.
	pub async fn version(&self, key: &RowKey) -> Result<Option<(u64, Option<Row>)>> {
		let snapshot = self.snapshot()?;
		let prefix = keys::row_prefix(&self.scope, key);
		let mut suffix = Vec::new();
		DescendingU64(self.as_of).pack_into(&mut suffix);
		let mut scan = snapshot
			.scan_prefix_with_options(
				prefix,
				(
					Bound::Included(Bytes::copy_from_slice(&suffix[1..])),
					Bound::<Bytes>::Unbounded,
				),
				&self.options(1),
			)
			.await?;
		let result = match scan.next().await? {
			None => None,
			Some(found) => {
				if found.key.len().saturating_add(found.value.len()) > self.config.max_read_bytes {
					return Err(Error::TooLarge);
				}
				let (scope, row, seq) = keys::decode_key(&found.key)?;
				if scope != self.scope || &row != key || seq > self.as_of {
					return Err(Error::Corrupt);
				}
				Some((seq, keys::decode_row(&row, found.value)?))
			}
		};
		self.check()?;
		Ok(result)
	}
	/// The stored versions of `key` in this volume's own layer at or below the session's
	/// position, newest first, as raw bytes beside their decoding: `Ok(None)` is a delete
	/// marker and `Err` a value that does not decode. Reads at most `limit` versions and fails
	/// as too large past the read byte bound. Chunk rows are refused, since their versions
	/// live in the chunk layout and can be large.
	pub async fn raw_versions(
		&self,
		key: &RowKey,
		limit: usize,
	) -> Result<Vec<(u64, Bytes, std::result::Result<Option<Row>, String>)>> {
		if matches!(key, RowKey::Chunk(..)) || limit > self.config.max_rows {
			return Err(Error::TooLarge);
		}
		let snapshot = self.snapshot()?;
		let prefix = keys::row_prefix(&self.scope, key);
		let mut suffix = Vec::new();
		DescendingU64(self.as_of).pack_into(&mut suffix);
		let mut scan = snapshot
			.scan_prefix_with_options(
				prefix,
				(
					Bound::Included(Bytes::copy_from_slice(&suffix[1..])),
					Bound::<Bytes>::Unbounded,
				),
				&self.options(limit.max(1)),
			)
			.await?;

		// Versions sort newest first under the descending sequence suffix.
		let mut versions = Vec::new();
		let mut bytes = 0usize;
		while versions.len() < limit {
			let Some(found) = scan.next().await? else {
				break;
			};
			bytes = bytes
				.saturating_add(found.key.len())
				.saturating_add(found.value.len());
			if bytes > self.config.max_read_bytes {
				return Err(Error::TooLarge);
			}
			let (scope, row, seq) = keys::decode_key(&found.key)?;
			if scope != self.scope || &row != key || seq > self.as_of {
				return Err(Error::Corrupt);
			}
			let decoded =
				keys::decode_row(&row, found.value.clone()).map_err(|error| format!("{error:#}"));
			versions.push((seq, found.value, decoded));
		}
		self.check()?;
		Ok(versions)
	}

	pub async fn get(&self, key: &RowKey) -> Result<Option<Row>> {
		if let RowKey::Chunk(ino, index) = key {
			return Ok(self
				.read_chunks(*ino, *index, 1)
				.await?
				.pop()
				.flatten()
				.map(|bytes| Row::Bytes(bytes.into())));
		}
		Ok(self.version(key).await?.and_then(|(_, value)| value))
	}
	pub async fn scan(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
	) -> Result<Vec<(RowKey, Row)>> {
		let snapshot = self.snapshot()?;
		if limit > self.config.max_rows {
			return Err(Error::TooLarge);
		}
		if limit == 0 {
			return Ok(Vec::new());
		}
		let (prefix, mut lower, upper) = range_bounds(&self.scope, range);
		if let Some(after) = after {
			if !range.contains(after) {
				return Err(Error::Corrupt);
			}
			let next = prefix_end(&keys::row_prefix(&self.scope, after)).ok_or(Error::Corrupt)?;
			if next < prefix || !next.starts_with(&prefix) {
				return Ok(Vec::new());
			}
			lower = Bound::Included(next.slice(prefix.len()..));
		}
		let mut scan = snapshot
			.scan_prefix_with_options(prefix.clone(), (lower, upper.clone()), &self.options(limit))
			.await?;
		let mut rows = Vec::new();
		let mut seen = None;
		let mut bytes = 0usize;
		while rows.len() < limit {
			let Some(found) = scan.next().await? else {
				break;
			};
			self.check()?;
			let Ok((scope, key, seq)) = keys::decode_key(&found.key) else {
				if matches!(range, RowRange::All | RowRange::MetadataAll) {
					let checkpoints = keys::CheckpointRowsPrefix::new(
						self.scope.hash,
						self.scope.name.clone(),
						self.scope.incarnation,
						0,
					)
					.pack();
					let holds = keys::ForkHoldRowsPrefix::new(
						self.scope.hash,
						self.scope.name.clone(),
						self.scope.incarnation,
						0,
					)
					.pack();
					if found.key.starts_with(&checkpoints) || found.key.starts_with(&holds) {
						let next = prefix_end(&holds).ok_or(Error::Corrupt)?;
						scan = snapshot
							.scan_prefix_with_options(
								prefix.clone(),
								(Bound::Included(next.slice(prefix.len()..)), upper.clone()),
								&self.options(limit - rows.len()),
							)
							.await?;
						continue;
					}
					if keys::IncarnationRecordKey::unpack(&found.key).is_ok()
						|| keys::VolumeIncarnationRecordKey::unpack(&found.key).is_ok()
					{
						continue;
					}
				}
				return Err(Error::Corrupt);
			};
			if scope != self.scope {
				return Err(Error::Corrupt);
			}
			if !range.contains(&key) || seq > self.as_of || seen.as_ref() == Some(&key) {
				continue;
			}
			seen = Some(key.clone());
			if let Some(value) = keys::decode_row(&key, found.value.clone())? {
				bytes = bytes
					.checked_add(found.key.len())
					.and_then(|n| n.checked_add(found.value.len()))
					.filter(|n| *n <= self.config.max_read_bytes)
					.ok_or(Error::TooLarge)?;
				rows.push((key, value));
			}
		}
		self.check()?;
		Ok(rows)
	}
	pub(super) fn check_chunk_count(&self, count: usize) -> Result<()> {
		self.check()?;
		if count > self.config.max_rows
			|| count.saturating_mul(depot_fs_core::CHUNK_BYTES) > self.config.max_read_bytes
		{
			return Err(Error::TooLarge);
		}
		Ok(())
	}

	/// Reuse one scan per contiguous run, stepping to each chunk's next row and seeking to its
	/// logical bound only past older history. Keep tombstones and sequence-zero copies distinct from
	/// missing rows; the generic live-row scan cannot resolve fork inheritance.
	async fn chunk_versions(
		&self,
		ino: u64,
		indices: &[u64],
	) -> Result<Vec<Option<(u64, Option<Row>)>>> {
		let snapshot = self.snapshot()?;
		let prefix = keys::DataPrefix::new(
			self.scope.hash,
			self.scope.name.clone(),
			self.scope.incarnation,
			ino,
		)
		.pack();
		let address = |index, seq| {
			keys::ChunkKey::new(
				self.scope.hash,
				self.scope.name.clone(),
				self.scope.incarnation,
				ino,
				index,
				DescendingU64(seq),
			)
			.pack()
		};
		let mut ordered: Vec<_> = indices.iter().copied().zip(0..indices.len()).collect();
		ordered.sort_unstable();
		let mut result = vec![None; indices.len()];
		let mut rest = ordered.as_slice();
		while let Some(&(first, _)) = rest.first() {
			self.check()?;
			let mut count = 1;
			while count < rest.len() && rest[count].0.saturating_sub(rest[count - 1].0) <= 1 {
				count += 1;
			}
			let (run, remaining) = rest.split_at(count);
			rest = remaining;
			let lower = address(first, self.as_of);
			let upper = address(run[count - 1].0, 0);
			// A single chunk scans under its own row prefix, which the SST filters hash,
			// so the scan skips every SST that holds no version of it. A run spans
			// several rows and scans under the inode's data prefix, which only the optional
			// data prefix filter hashes; without it the run reads every SST whose key range
			// covers it.
			let single = keys::row_prefix(&self.scope, &RowKey::Chunk(ino, first));
			let scan_prefix = if count == 1 && lower.starts_with(&single) {
				single
			} else {
				prefix.clone()
			};
			// Include encoding overhead and retain fetched blocks, with a hard
			// read-ahead cap even when the session permits a larger response.
			let options = self
				.options(count)
				.with_order(slatedb::IterationOrder::Ascending)
				.with_read_ahead_bytes(
					count
						.saturating_mul(depot_fs_core::CHUNK_BYTES * 2)
						.clamp(64 * 1024, 4 * 1024 * 1024)
						.min(self.config.max_read_bytes),
				);
			let bulk = self.config.bulk;
			let is_bulk = bulk.min_chunks > 0 && count >= bulk.min_chunks;
			let options = if is_bulk {
				crate::metrics::FS_BULK_RUNS.inc();
				options
					.with_cache_blocks(bulk.cache_blocks)
					.with_max_fetch_tasks(bulk.fetch_tasks.max(1))
			} else {
				options
			};
			let mut scan = snapshot
				.scan_prefix_with_options(
					scan_prefix.clone(),
					lower.slice(scan_prefix.len()..)..=upper.slice(scan_prefix.len()..),
					&options,
				)
				.await?;
			let mut next = scan.next().await?;
			for &(index, slot) in run {
				self.check()?;
				let target = address(index, self.as_of);
				if next.as_ref().is_some_and(|row| row.key < target) {
					// The row after the preceding chunk's version is usually this chunk's
					// newest version, so one step is far cheaper than a seek, which rebuilds
					// the merge across every SST the run covers. Only when that row is still
					// before the target, an older version of the preceding chunk or a version
					// of this chunk newer than the read, seek past that history.
					next = scan.next().await?;
					if next.as_ref().is_some_and(|row| row.key < target) {
						scan.seek(&target).await?;
						next = scan.next().await?;
					}
				}
				let Some(found) = &next else { break };
				if found.key.len().saturating_add(found.value.len()) > self.config.max_read_bytes {
					return Err(Error::TooLarge);
				}
				let (scope, row, seq) = keys::decode_key(&found.key)?;
				let RowKey::Chunk(found_ino, found_index) = &row else {
					return Err(Error::Corrupt);
				};
				if scope != self.scope || *found_ino != ino || *found_index < index {
					return Err(Error::Corrupt);
				}
				if *found_index == index {
					if seq > self.as_of {
						return Err(Error::Corrupt);
					}
					result[slot] = Some((seq, keys::decode_row(&row, found.value.clone())?));
				}
				// Retain lookahead when this chunk is absent or requested twice.
			}
		}
		self.check()?;
		Ok(result)
	}

	/// Batched local-layer lookups per contiguous run. Missing chunks in a fully
	/// materialized image are holes and stop a descendant's stale chain walk.
	pub async fn read_local_chunks(
		&self,
		ino: u64,
		indices: &[u64],
	) -> Result<Vec<depot_fs_core::forks::ChunkLookup>> {
		use depot_fs_core::forks::{ChunkLookup, chunk_lookup};
		self.check_chunk_count(indices.len())?;
		let inode = match self
			.version(&RowKey::Inode(ino))
			.await?
			.and_then(|(_, row)| row)
		{
			Some(Row::Inode(inode)) => inode,
			None => return Ok(vec![ChunkLookup::Resolved(None); indices.len()]),
			_ => return Err(Error::Corrupt),
		};
		let mut result = Vec::with_capacity(indices.len());
		for (index, version) in indices.iter().zip(self.chunk_versions(ino, indices).await?) {
			let value = chunk_lookup(&inode, *index, version).map_err(|_| Error::Corrupt)?;
			result.push(
				if matches!(value, ChunkLookup::Base { .. }) && self.base_chain.is_empty() {
					ChunkLookup::Resolved(None)
				} else {
					value
				},
			);
		}
		self.check()?;
		Ok(result)
	}
}
/// Whether a session may register below the retention cold position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BelowCold {
	/// At a held checkpoint, whose versions the catalog keeps. Its registration holds the
	/// volume's stripe, so the writers that collect checkpoint images order against it.
	Checkpoints,
	/// Never: a claim read takes no stripe, so it reads only what cold or its pin keeps.
	Refused,
}

impl FilesystemStore {
	/// The core fixes applied before this effect. Even if apply progresses while
	/// the SlateDB snapshot opens, the logical bound hides those later versions.
	/// `record` may be stale, so the stored record decides whether the incarnation
	/// still exists and which base chain the session reads.
	pub async fn open_session(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: Option<u64>,
	) -> Result<Arc<ReadSession>> {
		// Hold registration and snapshot capture cannot race physical sweeping: the core runs
		// no sweep or other writer of the volume beside an open, so in `core` mode the open
		// takes only the tablet lock.
		let _running = self.enter_volumes([(
			super::locks::StoreVolume::from(&key.0.volume_key),
			super::locks::VolumeAccess::Read,
		)]);
		let _ordered = self
			.lock_core_ordered(
				super::locks::LockRequest::shared().read(super::locks::stripe(&key.0.volume_key)),
				LockOperation::OpenSession,
			)
			.await;
		let base_chain = match self
			.incarnation_row(&key.0.volume_key, record.volume_incarnation)
			.await?
		{
			Some(current) if current.status == FsVolumeStatus::Deleted => {
				return Err(Error::NotFound);
			}
			None if record.cold.volume_seq != 0 || !record.base_chain.is_empty() => {
				return Err(Error::NotFound);
			}
			Some(current) => current.base_chain,
			None => record.base_chain.clone(),
		};
		self.register_checked(
			key,
			record,
			applied,
			as_of,
			base_chain,
			BelowCold::Checkpoints,
		)
		.await
	}

	/// Opens a session from the maintainer's own record without reading the stored one.
	/// The caller guarantees `record` is current: the core orders the open after every
	/// store write of the volume, so the stored record cannot differ from it.
	pub(crate) async fn open_ordered_session(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: Option<u64>,
	) -> Result<Arc<ReadSession>> {
		// Hold registration and snapshot capture cannot race physical sweeping.
		let _ordered = self
			.lock(
				super::locks::LockRequest::shared().read(super::locks::stripe(&key.0.volume_key)),
				LockOperation::OpenSession,
			)
			.await;
		self.register_checked(
			key,
			record,
			applied,
			as_of,
			record.base_chain.clone(),
			BelowCold::Checkpoints,
		)
		.await
	}

	/// Opens the one-request session of a claim read at `as_of`, against the record and
	/// applied position the core captured when it dispatched the read, and takes no lock.
	///
	/// A claim read never reads below the retention cold position, not even at a checkpoint,
	/// so it never depends on a version that only a checkpoint, a hold or a pin keeps. Every
	/// version it reads is therefore kept by the cold position checked under the catalog
	/// lock, or by the pin it registers there, and nothing a stripe holder collects is one of
	/// them: hold releases, checkpoint deletions, sweeps and a flush stage's retention
	/// deletions only remove versions below cold that no live pin protects. Taking no stripe
	/// keeps the read from waiting behind a flush stage whose writes stall on unflushed bytes
	/// or L0. A record change the captured record misses, such as a retired incarnation's
	/// sweep, is caught by the core's lifecycle check before the reply.
	pub(crate) async fn open_claim_session(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: u64,
	) -> Result<Arc<ReadSession>> {
		self.register_checked(
			key,
			record,
			applied,
			Some(as_of),
			record.base_chain.clone(),
			BelowCold::Refused,
		)
		.await
	}

	/// Validates the session's bounds against `record` and registers it. The caller holds
	/// the volume's stripe, except for a claim read, which `below_cold` refuses below cold.
	async fn register_checked(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: Option<u64>,
		base_chain: Vec<FsBaseLink>,
		below_cold: BelowCold,
	) -> Result<Arc<ReadSession>> {
		self.admit()?;
		if record.volume_incarnation != key.0.volume_incarnation
			|| record.partition_key != key.0.volume_key.partition_key
			|| record.name != key.0.volume_key.name
			|| record.status == FsVolumeStatus::Deleted
		{
			return Err(Error::NotFound);
		}
		let as_of = as_of.unwrap_or(applied.volume_seq);
		if as_of > applied.volume_seq {
			return Err(Error::Lagging);
		}
		if applied.volume_seq < record.cold.volume_seq {
			return Err(Error::Corrupt);
		}
		self.register_session(record, applied, as_of, base_chain, below_cold)
			.await
	}

	/// A held checkpoint remains readable after its logical name is deleted or
	/// restored. Registration shares the sweep lock, so releasing the durable
	/// hold cannot collect an already admitted reader's image.
	pub async fn open_checkpoint_session(&self, link: &FsBaseLink) -> Result<Arc<ReadSession>> {
		// The core does not order these opens; the stripe alone keeps a hold release,
		// checkpoint deletion or sweep of the volume from collecting the image.
		let _ordered = self
			.lock(
				super::locks::LockRequest::shared().read(super::locks::stripe(&link.volume_key)),
				LockOperation::OpenSession,
			)
			.await;
		self.admit()?;
		let scope = keys::Scope::new(&link.volume_key, Uuid::from_bytes(link.incarnation));
		let Some(point) = self
			.inspect_checkpoint(&scope, &link.checkpoint_name)
			.await?
			.checkpoint
		else {
			return Err(Error::NotFound);
		};
		if point.position != link.position {
			return Err(Error::Corrupt);
		}
		let Some(record) = self
			.incarnation_record(&link.volume_key, link.incarnation)
			.await?
		else {
			return Err(Error::NotFound);
		};
		// A fork pins its parent at the applied position, which may be above the parent's
		// stored cold position until an ordinary pass. The store serves the pin once the
		// parent's apply holds it, and until then, as after a maintainer move whose replay has
		// not reached it, the read is retryable.
		if record.cold.volume_seq < link.position.volume_seq {
			let applied = self
				.catalog
				.lock()
				.get(&scope)
				.and_then(|retention| retention.apply_cold.clone());
			if applied.is_none_or(|applied| applied.volume_seq < link.position.volume_seq) {
				return Err(Error::Lagging);
			}
		}
		if !point.checkpoint.named && !point.checkpoint.retention {
			let prefix = keys::CheckpointForkHoldsPrefix::new(
				scope.hash,
				scope.name.clone(),
				scope.incarnation,
				0,
				link.position.volume_seq,
				link.checkpoint_name.clone().into(),
			)
			.pack();
			if self
				.db
				.scan_prefix(prefix, ..)
				.await?
				.next()
				.await?
				.is_none()
			{
				return Err(Error::NotFound);
			}
		}
		self.register_session(
			&record,
			link.position.clone(),
			link.position.volume_seq,
			record.base_chain.clone(),
			BelowCold::Checkpoints,
		)
		.await
	}

	/// The caller holds mutation across validation, retention registration and
	/// snapshot capture. The fresh catalog decides whether an old point survives.
	pub(super) async fn register_session(
		&self,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: u64,
		base_chain: Vec<FsBaseLink>,
		below_cold: BelowCold,
	) -> Result<Arc<ReadSession>> {
		let scope = keys::Scope::new(
			&record.volume_key(),
			Uuid::from_bytes(record.volume_incarnation),
		);
		if !self.catalog.lock().contains_key(&scope) {
			self.load_retention(record).await?;
		}
		let now = self.clock.observe().map_err(|_| Error::Fenced)?.boot_ms;
		self.expire_sessions(now);
		let slot = self
			.session_slots
			.clone()
			.try_acquire_owned()
			.map_err(|_| Error::Overloaded)?;
		self.observe_session_usage();
		let id = self.rng.clone().uuid_v4();
		let expires = now.saturating_add(self.config.max_read_session_ms);
		{
			let mut catalog = self.catalog.lock();
			let retention = catalog.get_mut(&scope).ok_or(Error::Corrupt)?;
			let kept = match below_cold {
				BelowCold::Checkpoints => retention.checkpoints.contains(&as_of),
				BelowCold::Refused => false,
			};
			if as_of < retention.cold && !kept {
				return Err(Error::TooOld);
			}
			if retention.sessions.insert(id, (as_of, expires)).is_some() {
				return Err(Error::Corrupt);
			}
		}
		let pin = Pin {
			catalog: self.catalog.clone(),
			scope: scope.clone(),
			id,
		};
		let db = self.db.snapshot().await?;
		self.admit()?;
		let session = Arc::new(ReadSession {
			id,
			scope,
			applied,
			as_of,
			cold_volume_seq: record.cold.volume_seq,
			base_chain,
			base_reader: self.base_reader.clone(),
			expires_boot_ms: expires,
			idle_deadline_boot_ms: AtomicU64::new(
				now.saturating_add(self.config.read_session_idle_ms)
					.min(expires),
			),
			latest: std::sync::atomic::AtomicBool::new(false),
			snapshot: Mutex::new(Some(Snapshot {
				db,
				_pin: Some(pin),
				_slot: Some(slot),
				_usage: Some({
					use rivet_metrics::GaugeGuardExt;
					crate::metrics::READ_SESSIONS.inc_guard()
				}),
			})),
			clock: self.clock.clone(),
			guard: self.guard.clone(),
			config: self.config.clone(),
		});
		session.check()?;
		let mut sessions = self.sessions.lock();
		if sessions.len() >= self.config.max_sessions {
			return Err(Error::Overloaded);
		}
		sessions.insert(id, session.clone());
		Ok(session)
	}
	/// Warns once when held read sessions reach four fifths of the cap, and re-arms below half
	/// of it, so usage held near the cap logs once per episode.
	fn observe_session_usage(&self) {
		let cap = self.config.max_sessions;
		let used = cap.saturating_sub(self.session_slots.available_permits());
		if used.saturating_mul(5) >= cap.saturating_mul(4) {
			if !self.session_warned.swap(true, Ordering::AcqRel) {
				tracing::warn!(
					used,
					cap,
					"filesystem read sessions are near their cap; opens past it are refused as overloaded. Raise storage.max_read_sessions or lower storage.read_session_idle_ms"
				);
			}
		} else if used.saturating_mul(2) < cap {
			self.session_warned.store(false, Ordering::Release);
		}
	}
	pub fn session(&self, id: Uuid, key: &VolumeLogKey) -> Result<Arc<ReadSession>> {
		let session = self
			.sessions
			.lock()
			.get(&id)
			.cloned()
			.ok_or(Error::TooOld)?;
		if session.scope
			!= keys::Scope::new(
				&key.0.volume_key,
				Uuid::from_bytes(key.0.volume_incarnation),
			) {
			return Err(Error::Corrupt);
		}
		session.check()?;
		// A read keeps its session registered for another idle period.
		if let Ok(now) = self.clock.observe() {
			session.touch(now.boot_ms);
		}
		Ok(session)
	}
	/// Revokes every session past its deadline, or every session when this store may no longer
	/// serve. The grant is checked once per call, and only expired sessions are visited, so the
	/// cost does not grow with the number of live sessions.
	pub fn expire_sessions(&self, now_boot_ms: u64) {
		let may_serve = self.guard.may_send();
		let mut sessions = self.sessions.lock();
		if !may_serve {
			sessions.revoke_all();
			return;
		}
		sessions.expire(now_boot_ms);
	}
	pub(crate) fn close_session_for(&self, id: Uuid, key: &VolumeLogKey) -> Result<()> {
		self.admit()?;
		let mut sessions = self.sessions.lock();
		if let Some(session) = sessions.get(&id) {
			if session.scope
				!= keys::Scope::new(
					&key.0.volume_key,
					Uuid::from_bytes(key.0.volume_incarnation),
				) {
				return Err(Error::Corrupt);
			}
		}
		if let Some(session) = sessions.remove(&id) {
			session.revoke();
		}
		self.admit()?;
		Ok(())
	}
	pub fn close_session(&self, id: Uuid) {
		if let Some(session) = self.sessions.lock().remove(&id) {
			session.revoke();
		}
	}
}
fn prefix_end(prefix: &[u8]) -> Option<Bytes> {
	let mut next = prefix.to_vec();
	while let Some(last) = next.pop() {
		if last != 255 {
			next.push(last + 1);
			return Some(next.into());
		}
	}
	None
}
fn range_bounds(scope: &keys::Scope, range: &RowRange) -> (Bytes, Bound<Bytes>, Bound<Bytes>) {
	let (hash, name, incarnation) = (scope.hash, scope.name.clone(), scope.incarnation);
	let prefix = match range {
		RowRange::All => keys::IncarnationPrefix::new(hash, name, incarnation).pack(),
		RowRange::MetadataAll => keys::MetadataSpacePrefix::new(hash, name, incarnation).pack(),
		RowRange::ChunksAll => keys::DataSpacePrefix::new(hash, name, incarnation).pack(),
		RowRange::Metadata(ino) => keys::MetadataPrefix::new(hash, name, incarnation, *ino).pack(),
		RowRange::Chunks(ino) | RowRange::ChunkSuffix { ino, .. } => {
			keys::DataPrefix::new(hash, name, incarnation, *ino).pack()
		}
		RowRange::Directory(ino) => {
			keys::DirectoryRowsPrefix::new(hash, name, incarnation, *ino).pack()
		}
		RowRange::Xattrs(ino) => keys::XattrRowsPrefix::new(hash, name, incarnation, *ino).pack(),
		RowRange::Kv { ino, .. } => keys::KvRowsPrefix::new(hash, name, incarnation, *ino).pack(),
		RowRange::Orphans => keys::OrphanRowsPrefix::new(hash, name, incarnation, 0).pack(),
	};
	let (lower, upper) = if let RowRange::Kv { start, end, .. } = range {
		let mut lower = Vec::new();
		Bytes::copy_from_slice(start).pack_into(&mut lower);
		let upper = end.as_ref().map_or(Bound::Unbounded, |end| {
			let mut bytes = Vec::new();
			Bytes::copy_from_slice(end).pack_into(&mut bytes);
			Bound::Excluded(Bytes::from(bytes))
		});
		(Bound::Included(Bytes::from(lower)), upper)
	} else if let RowRange::ChunkSuffix { first, .. } = range {
		let mut lower = Vec::new();
		first.pack_into(&mut lower);
		(Bound::Included(Bytes::from(lower)), Bound::Unbounded)
	} else {
		(Bound::Unbounded, Bound::Unbounded)
	};
	(prefix, lower, upper)
}
