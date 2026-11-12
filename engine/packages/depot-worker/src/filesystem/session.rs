//! Fixed SlateDB snapshots and filesystem sequence bounds with expiring, grant-bound handles.

use super::filter::PinnedCut;
use super::{
	filter::Catalog,
	store::{FilesystemStore, LockOperation},
};
use bytes::Bytes;
use depot_fs_core::{
	FsBaseLink, FsVolumeRecord, FsVolumeStatus,
	keys::slatedb as keys,
	overlay::{self, Level, RangeWhiteout},
	store::{Row, RowKey, RowRange},
	wire::decode_base_row,
};
use depot_types::VolumeLogKey;
use parking_lot::Mutex;
use rivet_data::generated::shared_depot_fs_read_v2::{
	FsBaseGet, FsBaseRead, FsBaseRowsPage, FsBaseScan,
};
use rivet_data::generated::{shared_depot_fs_v2::FsCut, shared_depot_types_v1::VolumeLogPosition};
use rivet_env::{Clock, MutationGuard};
use rivet_keys::keys::{
	KeyAddress, PrefixAddress,
	codec::{BinaryComponent, DescendingU64, FormalKey},
};
use slatedb::{DbSnapshot, IterationOrder, config::ScanOptions};
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

/// One base link's answer to a batched get: by position in the asked keys, its version of each
/// key it holds (`Some(None)` a whiteout), and the error that ended its answer with the first
/// position it left unanswered. A position before that error with no version is a key the link
/// holds none of.
struct LinkAnswer {
	found: Vec<Option<Option<Row>>>,
	failed: Option<(usize, Error)>,
}

/// Matches an ancestor's page answering `chunk` to positions in `chunk`, and answers how many
/// keys of `chunk`, from its start, the page covered. The page answers keys in request order,
/// one row per asked position it holds a version of, so one forward cursor matches every row.
/// A complete page covers the whole chunk; an incomplete one covers through `next`, the last key
/// the ancestor examined, and must cover at least one key so every page makes progress.
fn match_base_page(
	chunk: &[RowKey],
	page: FsBaseRowsPage,
) -> Result<(Vec<(usize, Option<Row>)>, usize)> {
	if page.rows.len() > chunk.len() {
		return Err(Error::Corrupt);
	}
	let mut rows = Vec::with_capacity(page.rows.len());
	let mut cursor = 0;
	for row in page.rows {
		let (key, value) = decode_base_row(row).map_err(|_| Error::Corrupt)?;
		while cursor < chunk.len() && chunk[cursor] != key {
			cursor += 1;
		}
		if cursor == chunk.len() {
			return Err(Error::Corrupt);
		}
		rows.push((cursor, value));
		cursor += 1;
	}
	if page.complete {
		return Ok((rows, chunk.len()));
	}

	// The ancestor stopped at its own bounds. The examined keys are a prefix of the chunk ending
	// at `next`, at or after the last row's position; the first match from there is never past
	// the true one, so a duplicate asked twice is at worst asked again.
	let next = RowKey::from(page.next.ok_or(Error::Corrupt)?);
	let from = rows.last().map_or(0, |(position, _)| *position);
	let examined = chunk[from..]
		.iter()
		.position(|key| key == &next)
		.ok_or(Error::Corrupt)?;
	Ok((rows, from + examined + 1))
}

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
	/// The most metadata bytes a group fork acquire carries inline in its reply; a volume
	/// with more reads its metadata through the base chain.
	pub inline_metadata_bytes: usize,
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
			inline_metadata_bytes: 256 * 1024,
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
	/// Where the session reads claim log 0.
	pub as_of: u64,
	/// The cut the session reads at over a volume with several claims, one position per claim
	/// log, or `None` for a session at one sequence, `as_of`.
	pub cut: Option<FsCut>,
	/// The sequence each claim log is read at, from `cut`. A version is then visible only when
	/// this names its log at or above it. Without a cut every version at or below `as_of` is,
	/// which on a volume with one claim is claim log 0's history and on any volume what an
	/// apply reads of its own log's rows.
	pinned: Option<PinnedCut>,
	/// The volume's cold position when the session opened. A session at or below it reads
	/// only SSTs; above it, its snapshot also holds applied rows in the memtable.
	pub cold_volume_seq: u64,
	/// Chain captured with this snapshot; materialization never rewrites an existing session.
	pub base_chain: Vec<FsBaseLink>,
	/// Whether the volume holds its own copy of every metadata row, so the chain is consulted
	/// for chunks only; a lazily forked volume whose metadata was too large to copy reads its
	/// metadata through the chain as well.
	pub metadata_local: bool,
	pub(super) base_reader: Option<Arc<dyn super::base::BaseLayerReader>>,
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
			cut: None,
			pinned: None,
			// Internal sessions never answer client reads, so no read metric uses this.
			cold_volume_seq: applied.volume_seq,
			base_chain: Vec::new(),
			metadata_local: true,
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

	/// The newest sequence any claim log is read at, where a scan of versions starts.
	fn newest(&self) -> u64 {
		match &self.pinned {
			Some(pinned) => pinned.0.values().copied().max().unwrap_or(self.as_of),
			None => self.as_of,
		}
	}

	/// The position the session reads at as a cut: its cut over several claim logs, or its
	/// applied position on claim log 0.
	pub(crate) fn applied_cut(&self) -> FsCut {
		match &self.cut {
			Some(cut) => cut.clone(),
			None => FsCut::single(self.applied.clone()),
		}
	}

	/// Whether the version at `seq` written by `claim_log` is visible to this session.
	pub(crate) fn sees(&self, claim_log: u16, seq: u64) -> bool {
		match &self.pinned {
			Some(pinned) => pinned.contains(claim_log, seq),
			None => seq <= self.as_of,
		}
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
	/// Whether a read of `key` on this volume consults the base chain when the volume holds no
	/// version of it: a metadata row of a volume that reads its metadata through its lineage.
	/// Chunks have their own chain walk; claims, orphans and range whiteouts belong to their
	/// own level.
	fn reads_through(&self, key: &RowKey) -> bool {
		!self.base_chain.is_empty() && !self.metadata_local && reads_through_kind(key)
	}

	/// Whether a scan of `range` on this volume merges the base chain's rows into its own.
	fn range_reads_through(&self, range: &RowRange) -> bool {
		if self.base_chain.is_empty() || self.metadata_local {
			return false;
		}
		match range {
			RowRange::All
			| RowRange::MetadataAll
			| RowRange::Metadata(_)
			| RowRange::Directory(_)
			| RowRange::Xattrs(_)
			| RowRange::Kv { .. } => true,
			RowRange::ChunksAll
			| RowRange::Chunks(_)
			| RowRange::ChunkSuffix { .. }
			| RowRange::Orphans
			| RowRange::KvRangeWhiteouts(_)
			| RowRange::KvRangeWhiteoutsAll => false,
		}
	}

	/// The newest version of `key` this session sees: this volume's own, or, for a metadata
	/// row a lazily forked volume reads through its lineage, the nearest base link's at its
	/// cut. Preserves Deleted versus absent so fork chunk lookup can stop at an explicit mask,
	/// and a whiteout of this volume or of a nearer link reads as Deleted. An inherited row
	/// carries sequence 0 and claim log 0: it was written by no entry of this volume.
	pub async fn version(&self, key: &RowKey) -> Result<Option<(u64, u16, Option<Row>)>> {
		let mut versions = self.versions(std::slice::from_ref(key)).await?;
		versions.pop().ok_or(Error::Corrupt)
	}

	/// The newest version of each of `keys` this session sees, in order, as [`Self::version`]
	/// answers one. The keys this volume does not hold and no range whiteout of its own hides
	/// are asked of every base link at once, in one batched get per link, and each takes the
	/// nearest link's answer: an inherited miss costs one parallel round of one request per link
	/// instead of a serial walk per key. Only the own whiteouts that can span the keys are read.
	pub async fn versions(&self, keys: &[RowKey]) -> Result<Vec<Option<(u64, u16, Option<Row>)>>> {
		let mut versions = Vec::with_capacity(keys.len());
		let mut absent: Vec<usize> = Vec::new();
		for (slot, key) in keys.iter().enumerate() {
			let local = self.local_version(key).await?;
			if local.is_none() && self.reads_through(key) {
				absent.push(slot);
			}
			versions.push(local);
		}

		// A range whiteout of this volume hides a key it does not hold from every link.
		let absent_keys: Vec<RowKey> = absent.iter().map(|slot| keys[*slot].clone()).collect();
		let hidden = self.hidden_by_own_whiteouts(&absent_keys).await?;
		let through: Vec<usize> = absent
			.into_iter()
			.zip(hidden)
			.filter(|(_, hidden)| !hidden)
			.map(|(slot, _)| slot)
			.collect();
		if through.is_empty() {
			return Ok(versions);
		}

		// Every link answers the same keys in parallel, and each link is joined even when another
		// fails: a key takes the nearest link's answer, and a link's error fails the read only for
		// a key no nearer link answered.
		let reader = self.base_reader.as_ref().ok_or(Error::BaseUnavailable)?;
		self.check()?;
		let wanted: Vec<RowKey> = through.iter().map(|slot| keys[*slot].clone()).collect();
		let mut answers = futures_util::future::join_all(
			self.base_chain
				.iter()
				.map(|link| self.base_get(reader.as_ref(), link, &wanted)),
		)
		.await;
		self.check()?;
		for (index, slot) in through.into_iter().enumerate() {
			for answer in answers.iter_mut() {
				if let Some(value) = answer.found[index].take() {
					versions[slot] = Some((0, rivet_data::ROOT_CLAIM_LOG, value.map(inherited)));
					break;
				}
				let unanswered = answer
					.failed
					.as_ref()
					.is_some_and(|(from, _)| index >= *from);
				if unanswered {
					if let Some((_, error)) = answer.failed.take() {
						return Err(error);
					}
				}
			}
		}
		Ok(versions)
	}

	/// What one base link answers for `keys` at its cut: its version of each key it holds, and
	/// the error that cut its answer short. A link answers in pages of at most this session's row
	/// limit, and an ancestor may answer fewer keys than asked, at its own row limit or read byte
	/// budget, so the rest is asked again from the first key it did not answer. Never fails as a
	/// whole: the keys answered before an error keep their answers, so a nearer link's answers
	/// are never lost to a deeper link's refusal.
	async fn base_get(
		&self,
		reader: &dyn super::base::BaseLayerReader,
		link: &FsBaseLink,
		keys: &[RowKey],
	) -> LinkAnswer {
		let mut answer = LinkAnswer {
			found: vec![None; keys.len()],
			failed: None,
		};
		let mut first = 0;
		while first < keys.len() {
			let end = keys
				.len()
				.min(first.saturating_add(self.config.max_rows.max(1)));
			let chunk = &keys[first..end];
			let page = reader
				.rows(
					link,
					FsBaseRead::FsBaseGet(FsBaseGet {
						keys: chunk.iter().cloned().map(Into::into).collect(),
					}),
					self.expires_boot_ms,
				)
				.await;
			let covered = page.and_then(|page| match_base_page(chunk, page));
			match covered {
				Ok((rows, covered)) => {
					for (offset, value) in rows {
						answer.found[first + offset] = Some(value);
					}
					first += covered;
				}
				Err(error) => {
					answer.failed = Some((first, error));
					break;
				}
			}
		}
		answer
	}

	/// Every range whiteout this volume's own layer holds in `range`, which must be a range of
	/// whiteout rows, read a page at a time. A volume holds at most one whiteout per range
	/// delete it applied, and reading them never fails on how many there are.
	async fn own_whiteouts(&self, range: &RowRange) -> Result<Vec<RangeWhiteout>> {
		let page = self.config.max_rows.max(1);
		let mut whiteouts = Vec::new();
		let mut cursor = None;
		loop {
			let rows = self
				.scan_inner(range, cursor.as_ref(), page, true, false)
				.await?;
			let Some((last, _, _)) = rows.last() else {
				return Ok(whiteouts);
			};
			cursor = Some(last.clone());
			whiteouts.extend(rows.into_iter().filter_map(|(key, _, value)| {
				value.and_then(|value| RangeWhiteout::from_row(&key, &value))
			}));
		}
	}

	/// The range whiteout of this volume's own layer in KV file `ino` with the greatest start at
	/// or before `key`. A file's whiteouts are disjoint, so it is the only one that can span `key`
	/// or touch a range starting there. Reads back from `key` to the first start whose newest
	/// version this session sees is a whiteout, passing over the starts a later range delete
	/// coalesced away.
	pub(super) async fn own_whiteout_at_or_before(
		&self,
		ino: u64,
		key: &[u8],
	) -> Result<Option<RangeWhiteout>> {
		let snapshot = self.snapshot()?;
		let range = RowRange::KvRangeWhiteouts(ino);
		let (prefix, _, _) = range_bounds(&self.scope, &range);
		let past = whiteout_versions_end(&self.scope, ino, key)?;
		let options = self.options(1).with_order(IterationOrder::Descending);
		let mut scan = snapshot
			.scan_prefix_with_options(
				prefix.clone(),
				(
					Bound::<Bytes>::Unbounded,
					Bound::Excluded(past.slice(prefix.len()..)),
				),
				&options,
			)
			.await?;

		// Versions of one start arrive oldest first, so the last one the session sees is the
		// start's newest, decided once the scan moves to an earlier start.
		let mut start: Option<RowKey> = None;
		let mut newest: Option<Option<Row>> = None;
		while let Some(found) = scan.next().await? {
			self.check()?;
			if found.key.len().saturating_add(found.value.len()) > self.config.max_read_bytes {
				return Err(Error::TooLarge);
			}
			let (scope, row, seq) = keys::decode_key(&found.key)?;
			if scope != self.scope || !range.contains(&row) {
				return Err(Error::Corrupt);
			}
			if start.as_ref() != Some(&row) {
				if let Some(whiteout) = live_whiteout(start.take(), newest.take()) {
					return Ok(Some(whiteout));
				}
				start = Some(row.clone());
			}
			if seq > self.newest() {
				continue;
			}
			let (claim_log, value) = keys::decode_stamped_row(&row, found.value)?;
			if self.sees(claim_log, seq) {
				newest = Some(value);
			}
		}
		self.check()?;
		Ok(live_whiteout(start, newest))
	}

	/// At most `limit` range whiteouts of this volume's own layer in KV file `ino` whose start is
	/// after `after` and, with `through`, at or before it, in start order. A page cut short at the
	/// read byte bound is not the end; an empty page is.
	pub(super) async fn own_whiteouts_after(
		&self,
		ino: u64,
		after: &[u8],
		through: Option<&[u8]>,
		limit: usize,
	) -> Result<Vec<RangeWhiteout>> {
		let snapshot = self.snapshot()?;
		let range = RowRange::KvRangeWhiteouts(ino);
		let (prefix, _, _) = range_bounds(&self.scope, &range);
		let lower = whiteout_versions_end(&self.scope, ino, after)?;
		let upper = match through {
			Some(through) => Bound::Excluded(
				whiteout_versions_end(&self.scope, ino, through)?.slice(prefix.len()..),
			),
			None => Bound::Unbounded,
		};
		let (rows, _) = self
			.scan_within(
				&snapshot,
				&range,
				prefix.clone(),
				(Bound::Included(lower.slice(prefix.len()..)), upper),
				limit.min(self.config.max_rows),
				true,
				false,
			)
			.await?;
		Ok(rows
			.into_iter()
			.filter_map(|(key, _, value)| {
				value.and_then(|value| RangeWhiteout::from_row(&key, &value))
			})
			.collect())
	}

	/// For each of `keys`, whether a range whiteout of this volume's own layer spans it; only a
	/// KV key can be. Per KV file it reads the whiteout at or before the file's first asked key
	/// and those starting after it up to the last, a page at a time as the keys reach them, so a
	/// read costs two seeks per file however many whiteouts the file holds elsewhere.
	pub(super) async fn hidden_by_own_whiteouts(&self, keys: &[RowKey]) -> Result<Vec<bool>> {
		let mut hidden = vec![false; keys.len()];
		let mut by_file: std::collections::BTreeMap<u64, Vec<(&[u8], usize)>> =
			std::collections::BTreeMap::new();
		for (slot, key) in keys.iter().enumerate() {
			if let RowKey::Kv(ino, key) = key {
				by_file
					.entry(*ino)
					.or_default()
					.push((key.as_slice(), slot));
			}
		}
		let page = self.config.max_rows.max(1);
		for (ino, mut wanted) in by_file {
			wanted.sort_unstable();
			let (Some((first, _)), Some((last, _))) = (wanted.first(), wanted.last()) else {
				continue;
			};
			let (first, last) = (first.to_vec(), last.to_vec());
			let mut covering = self.own_whiteout_at_or_before(ino, &first).await?;
			let mut ahead = std::collections::VecDeque::new();
			let mut cursor = first.clone();
			let mut exhausted = first == last;
			for (key, slot) in wanted {
				loop {
					if ahead.is_empty() && !exhausted {
						let next = self
							.own_whiteouts_after(ino, &cursor, Some(&last), page)
							.await?;
						// A page that reaches the last asked key is the final one: no start
						// after it can cover an asked key.
						match next.last() {
							Some(whiteout) if whiteout.start < last => {
								cursor = whiteout.start.clone();
							}
							Some(_) | None => exhausted = true,
						}
						ahead.extend(next);
					}
					let reached = ahead
						.front()
						.is_some_and(|next: &RangeWhiteout| next.start.as_slice() <= key);
					if !reached {
						break;
					}
					covering = ahead.pop_front();
				}
				hidden[slot] = covering
					.as_ref()
					.is_some_and(|whiteout| whiteout.covers(&keys[slot]));
			}
		}
		Ok(hidden)
	}

	/// This volume's own newest version of `key`, as `version` reads it without the chain.
	///
	/// The scan starts at the newest position the session reads any claim log at and passes
	/// over versions the session does not see, of another log or above that log's position,
	/// to the first it does. A session of one claim log sees the first version it meets.
	pub(super) async fn local_version(
		&self,
		key: &RowKey,
	) -> Result<Option<(u64, u16, Option<Row>)>> {
		let snapshot = self.snapshot()?;
		let prefix = keys::row_prefix(&self.scope, key);
		let mut suffix = Vec::new();
		DescendingU64(self.newest()).pack_into(&mut suffix);
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
		let mut result = None;
		while let Some(found) = scan.next().await? {
			if found.key.len().saturating_add(found.value.len()) > self.config.max_read_bytes {
				return Err(Error::TooLarge);
			}
			let (scope, row, seq) = keys::decode_key(&found.key)?;
			if scope != self.scope || &row != key || seq > self.newest() {
				return Err(Error::Corrupt);
			}
			let (claim_log, value) = keys::decode_stamped_row(&row, found.value)?;
			if self.sees(claim_log, seq) {
				result = Some((seq, claim_log, value));
				break;
			}
		}
		self.check()?;
		Ok(result)
	}
	/// The stored versions of `key` in this volume's own layer at or below the session's
	/// position, newest first, as raw bytes beside their decoding: `Ok(None)` is a delete
	/// marker and `Err` a value that does not decode. Each version names the claim log that
	/// wrote it, or claim log 0 when its value does not decode. Reads at most `limit` versions
	/// and fails as too large past the read byte bound. Chunk rows are refused, since their
	/// versions live in the chunk layout and can be large.
	pub async fn raw_versions(
		&self,
		key: &RowKey,
		limit: usize,
	) -> Result<Vec<(u64, u16, Bytes, std::result::Result<Option<Row>, String>)>> {
		if matches!(key, RowKey::Chunk(..)) || limit > self.config.max_rows {
			return Err(Error::TooLarge);
		}
		let snapshot = self.snapshot()?;
		let prefix = keys::row_prefix(&self.scope, key);
		let mut suffix = Vec::new();
		DescendingU64(self.newest()).pack_into(&mut suffix);
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
			if scope != self.scope || &row != key || seq > self.newest() {
				return Err(Error::Corrupt);
			}
			let (claim_log, decoded) = match keys::decode_stamped_row(&row, found.value.clone()) {
				Ok((claim_log, decoded)) => (claim_log, Ok(decoded)),
				Err(error) => (rivet_data::ROOT_CLAIM_LOG, Err(format!("{error:#}"))),
			};
			// Only the versions inside the session's cut are its own to report.
			if self.cut.is_some() && !self.sees(claim_log, seq) {
				continue;
			}
			versions.push((seq, claim_log, found.value, decoded));
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
		Ok(self.get_stamped(key).await?.map(|(_, value)| value))
	}
	/// The rows of `keys`, which must be metadata rows, in order, read through the lineage in one
	/// batch as [`Self::versions`] describes.
	pub async fn get_many(&self, keys: &[RowKey]) -> Result<Vec<Option<Row>>> {
		if keys.iter().any(|key| matches!(key, RowKey::Chunk(..))) {
			return Err(Error::Corrupt);
		}
		Ok(self
			.versions(keys)
			.await?
			.into_iter()
			.map(|version| version.and_then(|(_, _, value)| value))
			.collect())
	}

	/// A metadata row with the claim log that wrote its newest version.
	pub async fn get_stamped(&self, key: &RowKey) -> Result<Option<(u16, Row)>> {
		Ok(self
			.version(key)
			.await?
			.and_then(|(_, claim_log, value)| value.map(|value| (claim_log, value))))
	}
	/// Rows of `range` after `after`, each with the claim log that wrote its newest version.
	pub async fn scan(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
	) -> Result<Vec<(RowKey, u16, Row)>> {
		if self.range_reads_through(range) {
			return self.scan_overlay(range, after, limit).await;
		}
		// Fork/export callers interpret a short page as exhaustion, so retain the
		// strict byte-limit error rather than returning a byte-limited short page.
		Ok(present(
			self.scan_inner(range, after, limit, false, false).await?,
		))
	}

	/// This volume's own rows and whiteouts of `range` after `after`, as a base read of this
	/// layer answers them: a whiteout is a key whose newest version is a delete. A page may stop
	/// short at the read byte bound; the flag says whether it reached the end of the range.
	pub(super) async fn scan_versions(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
	) -> Result<(Vec<(RowKey, u16, Option<Row>)>, bool)> {
		let snapshot = self.snapshot()?;
		let Some((prefix, bounds)) = self.scan_bounds(range, after)? else {
			return Ok((Vec::new(), true));
		};
		self.scan_within(&snapshot, range, prefix, bounds, limit, true, true)
			.await
	}

	/// A page of `range` through the overlay: this volume's rows on top of each base link's at
	/// its cut, nearest first. Reads on until the page is full or every level is exhausted,
	/// since callers take a short page for the end of the range; a range that hides more than
	/// `OVERLAY_ROUNDS` pages of keys behind whiteouts is refused as too large.
	async fn scan_overlay(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
	) -> Result<Vec<(RowKey, u16, Row)>> {
		let reader = self.base_reader.as_ref().ok_or(Error::BaseUnavailable)?;
		if limit == 0 {
			return Ok(Vec::new());
		}
		// Each level's range whiteouts, this volume's first, gathered once for the scan where
		// the range may hold KV keys. Every link's are read at once, in one parallel round.
		let whiteout_range = match range {
			RowRange::Kv { ino, .. } | RowRange::Metadata(ino) => {
				Some(RowRange::KvRangeWhiteouts(*ino))
			}
			RowRange::All | RowRange::MetadataAll => Some(RowRange::KvRangeWhiteoutsAll),
			_ => None,
		};
		let mut whiteouts: Vec<Vec<RangeWhiteout>> = Vec::with_capacity(self.base_chain.len() + 1);
		match whiteout_range {
			Some(whiteout_range) => {
				let link_levels = self
					.base_chain
					.iter()
					.map(|link| self.link_whiteouts(reader.as_ref(), link, &whiteout_range));
				let (local, levels) = futures_util::future::try_join(
					self.own_whiteouts(&whiteout_range),
					futures_util::future::try_join_all(link_levels),
				)
				.await?;
				whiteouts.push(local);
				whiteouts.extend(levels);
			}
			None => whiteouts.resize_with(self.base_chain.len() + 1, Vec::new),
		}

		let mut rows = Vec::new();
		let mut cursor = after.cloned();
		for _ in 0..OVERLAY_ROUNDS {
			self.check()?;
			let wanted = limit - rows.len();
			let local = self
				.scan_inner(range, cursor.as_ref(), wanted, false, true)
				.await?;
			let mut claim_logs: std::collections::BTreeMap<RowKey, u16> = local
				.iter()
				.map(|(key, claim_log, _)| (key.clone(), *claim_log))
				.collect();
			let mut levels = vec![Level {
				complete: local.len() < wanted,
				rows: local
					.into_iter()
					.map(|(key, _, value)| (key, value))
					.collect(),
				whiteouts: whiteouts[0].clone(),
				last_key: None,
			}];
			let pages = futures_util::future::try_join_all(self.base_chain.iter().map(|link| {
				reader.rows(
					link,
					FsBaseRead::FsBaseScan(FsBaseScan {
						range: range.clone().into(),
						after: cursor.clone().map(Into::into),
						limit: wanted as u32,
					}),
					self.expires_boot_ms,
				)
			}))
			.await?;
			for (index, page) in pages.into_iter().enumerate() {
				let last_key = page.next.map(RowKey::from);
				let mut level_rows = Vec::with_capacity(page.rows.len());
				for row in page.rows {
					let (key, value) = decode_base_row(row).map_err(|_| Error::Corrupt)?;
					// A link's claims, orphans and whiteouts belong to its own level.
					if reads_through_kind(&key) {
						level_rows.push((key, value));
					}
				}
				levels.push(Level {
					rows: level_rows,
					complete: page.complete,
					whiteouts: whiteouts[index + 1].clone(),
					last_key,
				});
			}
			self.check()?;
			let merged = overlay::merge(&levels, wanted);
			for (key, row) in merged.rows {
				// A row this volume holds is its own; any other came through a link.
				let (claim_log, row) = match claim_logs.remove(&key) {
					Some(claim_log) => (claim_log, row),
					None => (rivet_data::ROOT_CLAIM_LOG, inherited(row)),
				};
				rows.push((key, claim_log, row));
			}
			if merged.complete || rows.len() >= limit {
				return Ok(rows);
			}
			match merged.next_after {
				Some(next) => cursor = Some(next),
				None => return Ok(rows),
			}
		}
		Err(Error::TooLarge)
	}

	/// Every range whiteout one base link holds in `range` at its cut, which must be a range of
	/// whiteout rows, asked a page at a time from where the last page ended until the link
	/// answers the range complete. A link's pages count the whiteouts it coalesced away too.
	async fn link_whiteouts(
		&self,
		reader: &dyn super::base::BaseLayerReader,
		link: &FsBaseLink,
		range: &RowRange,
	) -> Result<Vec<RangeWhiteout>> {
		let mut whiteouts = Vec::new();
		let mut after: Option<RowKey> = None;
		loop {
			let page = reader
				.rows(
					link,
					FsBaseRead::FsBaseScan(FsBaseScan {
						range: range.clone().into(),
						after: after.clone().map(Into::into),
						limit: self.config.max_rows as u32,
					}),
					self.expires_boot_ms,
				)
				.await?;
			for row in page.rows {
				let (key, value) = decode_base_row(row).map_err(|_| Error::Corrupt)?;
				if let Some(whiteout) =
					value.and_then(|value| RangeWhiteout::from_row(&key, &value))
				{
					whiteouts.push(whiteout);
				}
			}
			if page.complete {
				return Ok(whiteouts);
			}

			// An incomplete page names the last key it examined, which must move the cursor on.
			let next = RowKey::from(page.next.ok_or(Error::Corrupt)?);
			if !range.contains(&next) || after.as_ref().is_some_and(|after| &next <= after) {
				return Err(Error::Corrupt);
			}
			after = Some(next);
		}
	}

	/// Range-deletion replay continues until an empty page. It may receive a short
	/// page at the byte bound and resumes after the last returned key; the row that
	/// did not fit is read again on the next page. A single oversized row still fails.
	pub(super) async fn scan_for_apply(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
	) -> Result<Vec<(RowKey, u16, Row)>> {
		Ok(present(
			self.scan_inner(range, after, limit, true, false).await?,
		))
	}

	/// This volume's own rows of `range`, newest visible version of each key, with a delete
	/// marker as `None` when `include_whiteouts` is set and skipped otherwise.
	async fn scan_inner(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
		limit: usize,
		allow_partial_page: bool,
		include_whiteouts: bool,
	) -> Result<Vec<(RowKey, u16, Option<Row>)>> {
		let snapshot = self.snapshot()?;
		let Some((prefix, bounds)) = self.scan_bounds(range, after)? else {
			return Ok(Vec::new());
		};
		let (rows, _) = self
			.scan_within(
				&snapshot,
				range,
				prefix,
				bounds,
				limit,
				allow_partial_page,
				include_whiteouts,
			)
			.await?;
		Ok(rows)
	}

	/// The key prefix and the bounds within it of `range` after `after`, or `None` when nothing
	/// of the range is after it.
	fn scan_bounds(
		&self,
		range: &RowRange,
		after: Option<&RowKey>,
	) -> Result<Option<(Bytes, (Bound<Bytes>, Bound<Bytes>))>> {
		let (prefix, mut lower, upper) = range_bounds(&self.scope, range);
		if let Some(after) = after {
			if !range.contains(after) {
				return Err(Error::Corrupt);
			}
			let next = prefix_end(&keys::row_prefix(&self.scope, after)).ok_or(Error::Corrupt)?;
			if next < prefix || !next.starts_with(&prefix) {
				return Ok(None);
			}
			lower = Bound::Included(next.slice(prefix.len()..));
		}
		Ok(Some((prefix, (lower, upper))))
	}

	/// This volume's own rows between `bounds` under `prefix`, which must hold only rows of
	/// `range`, as [`Self::scan_inner`] reads them, and whether the scan reached the end of the
	/// bounds rather than stopping at `limit` or the read byte bound.
	#[allow(
		clippy::too_many_arguments,
		reason = "the scan's flags are the private knobs of its few callers"
	)]
	async fn scan_within(
		&self,
		snapshot: &DbSnapshot,
		range: &RowRange,
		prefix: Bytes,
		bounds: (Bound<Bytes>, Bound<Bytes>),
		limit: usize,
		allow_partial_page: bool,
		include_whiteouts: bool,
	) -> Result<(Vec<(RowKey, u16, Option<Row>)>, bool)> {
		if limit > self.config.max_rows {
			return Err(Error::TooLarge);
		}
		if limit == 0 {
			return Ok((Vec::new(), false));
		}
		// SlateDB refuses an empty range by panicking, and a range whose end is its start, or a
		// cursor at a range's last key, describes one.
		if bounds_empty(&bounds) {
			return Ok((Vec::new(), true));
		}
		let upper = bounds.1.clone();
		let mut scan = snapshot
			.scan_prefix_with_options(prefix.clone(), bounds, &self.options(limit))
			.await?;
		let mut rows = Vec::new();
		let mut seen = None;
		let mut bytes = 0usize;
		let mut ended = false;
		while rows.len() < limit {
			let Some(found) = scan.next().await? else {
				ended = true;
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
					let pins = keys::ForkPinRowsPrefix::new(
						self.scope.hash,
						self.scope.name.clone(),
						self.scope.incarnation,
						0,
					)
					.pack();
					// Checkpoint and hold rows sit together and are skipped in one seek; the
					// fork pin rows sit further on and get their own.
					let skip_to =
						if found.key.starts_with(&checkpoints) || found.key.starts_with(&holds) {
							Some(&holds)
						} else if found.key.starts_with(&pins) {
							Some(&pins)
						} else {
							None
						};
					if let Some(skip_to) = skip_to {
						let next = prefix_end(skip_to).ok_or(Error::Corrupt)?;
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
			if !range.contains(&key) || seq > self.newest() || seen.as_ref() == Some(&key) {
				continue;
			}
			let (claim_log, value) = keys::decode_stamped_row(&key, found.value.clone())?;
			// A version the session does not see leaves an older one of the row to read.
			if !self.sees(claim_log, seq) {
				continue;
			}
			seen = Some(key.clone());
			if value.is_none() && !include_whiteouts {
				continue;
			}
			let next_bytes = bytes
				.checked_add(found.key.len())
				.and_then(|n| n.checked_add(found.value.len()))
				.filter(|n| *n <= self.config.max_read_bytes);
			let Some(next_bytes) = next_bytes else {
				if allow_partial_page && !rows.is_empty() {
					break;
				}
				return Err(Error::TooLarge);
			};
			bytes = next_bytes;
			rows.push((key, claim_log, value));
		}
		self.check()?;
		Ok((rows, ended))
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
			let lower = address(first, self.newest());
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
				let target = address(index, self.newest());
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
					if seq > self.newest() {
						return Err(Error::Corrupt);
					}
					let (claim_log, value) = keys::decode_stamped_row(&row, found.value.clone())?;
					if self.sees(claim_log, seq) {
						result[slot] = Some((seq, value));
					} else {
						// Pass over the chunk's versions this session does not see. The
						// lookahead then rests on the first one it does, or past the chunk.
						let mut visible = None;
						while let Some(older) = scan.next().await? {
							let (_, older_row, older_seq) = keys::decode_key(&older.key)?;
							if older_row != row {
								next = Some(older);
								break;
							}
							let (claim_log, value) =
								keys::decode_stamped_row(&older_row, older.value.clone())?;
							if self.sees(claim_log, older_seq) {
								visible = Some((older_seq, value));
								next = Some(older);
								break;
							}
						}
						result[slot] = visible;
						continue;
					}
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
			.and_then(|(_, _, row)| row)
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

/// Whether a registered snapshot waits between requests or is closed by its current request.
#[derive(Clone, Copy)]
pub(super) enum SessionUse {
	Reusable,
	OneRequest,
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
		self.open_session_for(key, record, applied, as_of, SessionUse::Reusable)
			.await
	}

	pub(super) async fn open_session_for(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: Option<u64>,
		usage: SessionUse,
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
			usage,
		)
		.await
	}

	/// Opens a session from the maintainer's own record without reading the stored one.
	/// The caller guarantees `record` is current: the core orders the open after every
	/// store write of the volume, so the stored record cannot differ from it.
	pub(super) async fn open_ordered_session(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: Option<u64>,
		usage: SessionUse,
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
			usage,
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
			SessionUse::OneRequest,
		)
		.await
	}

	/// Validates the session's bounds against `record` and registers it. The caller holds
	/// the volume's stripe, except for a claim read, which `below_cold` refuses below cold.
	#[allow(
		clippy::too_many_arguments,
		reason = "The captured snapshot bounds, retention policy and request lifetime are validated together"
	)]
	async fn register_checked(
		&self,
		key: &VolumeLogKey,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: Option<u64>,
		base_chain: Vec<FsBaseLink>,
		below_cold: BelowCold,
		usage: SessionUse,
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
		self.register_session(record, applied, as_of, None, base_chain, below_cold, usage)
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
		if let Some(first_fork) = link.fork_pin {
			return self.open_fork_pin_session(link, scope, first_fork).await;
		}
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
		// The parent has one claim log, so the link's cut names claim log 0 alone.
		let link_position = link.position.root_position().ok_or(Error::Corrupt)?;
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
		if record.cold.volume_seq < link_position.volume_seq {
			let applied = self.catalog.lock().get(&scope).and_then(|retention| {
				retention
					.apply_cold
					.get(rivet_data::ROOT_CLAIM_LOG)
					.cloned()
			});
			if applied.is_none_or(|applied| applied.volume_seq < link_position.volume_seq) {
				return Err(Error::Lagging);
			}
		}
		if !point.checkpoint.named && !point.checkpoint.retention {
			let prefix = keys::CheckpointForkHoldsPrefix::new(
				scope.hash,
				scope.name.clone(),
				scope.incarnation,
				0,
				link_position.volume_seq,
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
			link_position.clone(),
			link_position.volume_seq,
			None,
			record.base_chain.clone(),
			BelowCold::Checkpoints,
			SessionUse::Reusable,
		)
		.await
	}

	/// A session at a group fork pin's cut, for a child that reads through the link. The pin
	/// row keyed by the link's root position and `first_fork` must hold the link's cut; a
	/// pin that left, because every child of its forks was archived, reads as not found.
	async fn open_fork_pin_session(
		&self,
		link: &FsBaseLink,
		scope: keys::Scope,
		first_fork: u64,
	) -> Result<Arc<ReadSession>> {
		let link_position = link.position.root_position().ok_or(Error::Corrupt)?;
		let key = keys::ForkPinKey::new(
			scope.hash,
			scope.name.clone(),
			scope.incarnation,
			0,
			link_position.volume_seq,
			first_fork,
		);
		let Some(pin) = self
			.db
			.get(key.pack())
			.await?
			.map(|bytes| key.deserialize(bytes))
			.transpose()?
		else {
			return Err(Error::NotFound);
		};
		if pin.cut != link.position {
			return Err(Error::Corrupt);
		}
		let Some(record) = self
			.incarnation_record(&link.volume_key, link.incarnation)
			.await?
		else {
			return Err(Error::NotFound);
		};
		// As for a checkpoint link: the pin may sit above the stored cold position until a
		// pass, and is served once the apply holds it.
		if record.cold.volume_seq < link_position.volume_seq {
			let applied = self.catalog.lock().get(&scope).and_then(|retention| {
				retention
					.apply_cold
					.get(rivet_data::ROOT_CLAIM_LOG)
					.cloned()
			});
			if applied.is_none_or(|applied| applied.volume_seq < link_position.volume_seq) {
				return Err(Error::Lagging);
			}
		}
		self.register_session(
			&record,
			link_position.clone(),
			link_position.volume_seq,
			Some(link.position.clone()),
			record.base_chain.clone(),
			BelowCold::Checkpoints,
			SessionUse::Reusable,
		)
		.await
	}

	/// The caller holds mutation across validation, retention registration and
	/// snapshot capture. The fresh catalog decides whether an old point survives.
	///
	/// `cut` reads a volume with several claims at one position per claim log, with
	/// `as_of` its claim log 0 position; without it the session reads claim log 0 at `as_of`.
	pub(super) async fn register_session(
		&self,
		record: &FsVolumeRecord,
		applied: VolumeLogPosition,
		as_of: u64,
		cut: Option<FsCut>,
		base_chain: Vec<FsBaseLink>,
		below_cold: BelowCold,
		usage: SessionUse,
	) -> Result<Arc<ReadSession>> {
		let pinned = cut
			.as_ref()
			.map_or_else(|| PinnedCut::single(as_of), PinnedCut::from_cut);
		if pinned.root_seq() != Some(as_of) {
			return Err(Error::Corrupt);
		}
		let reads = cut.as_ref().map(|_| pinned.clone());
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
			// Below a log's cold position only a pin keeps the versions the session reads.
			let kept = match (below_cold, &cut) {
				(BelowCold::Checkpoints, None) => retention.pins_root_at(as_of),
				(BelowCold::Checkpoints, Some(_)) => retention.pins.contains(&pinned),
				(BelowCold::Refused, _) => false,
			};
			let below = pinned
				.0
				.iter()
				.any(|(claim_log, seq)| *seq < retention.cold_of(*claim_log));
			if below && !kept {
				return Err(Error::TooOld);
			}
			if retention
				.sessions
				.insert(id, (pinned.clone(), expires))
				.is_some()
			{
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
		// A one-request session has no idle phase: asynchronous storage work is part of
		// that request. Its fixed lifetime and grant still bound admission and the reply.
		let idle_deadline = match usage {
			SessionUse::OneRequest => expires,
			SessionUse::Reusable => self
				.clock
				.observe()
				.map_err(|_| Error::Fenced)?
				.boot_ms
				.saturating_add(self.config.read_session_idle_ms)
				.min(expires),
		};
		let session = Arc::new(ReadSession {
			id,
			scope,
			applied,
			as_of,
			cut,
			pinned: reads,
			cold_volume_seq: record.cold.volume_seq,
			base_chain,
			metadata_local: record.metadata_local,
			base_reader: self.base_reader.clone(),
			expires_boot_ms: expires,
			idle_deadline_boot_ms: AtomicU64::new(idle_deadline),
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
	/// Revokes every read session of the volumes `keys` name, as an archive of their group
	/// does. Visits the session table once, which its cap bounds; returns how many it revoked.
	pub fn revoke_sessions_of(&self, keys: &[VolumeLogKey]) -> usize {
		let scopes: std::collections::BTreeSet<keys::Scope> = keys
			.iter()
			.map(|key| {
				keys::Scope::of_log(
					&key.0.volume_key,
					Uuid::from_bytes(key.0.volume_incarnation),
				)
				.0
			})
			.collect();
		let mut sessions = self.sessions.lock();
		let revoked: Vec<Uuid> = sessions
			.values()
			.filter(|session| scopes.contains(&session.scope))
			.map(|session| session.id)
			.collect();
		for id in &revoked {
			if let Some(session) = sessions.remove(id) {
				session.revoke();
			}
		}
		revoked.len()
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
/// The first key past every version of the range whiteout of KV file `ino` starting at `start`,
/// and before the whiteout of any later start.
fn whiteout_versions_end(scope: &keys::Scope, ino: u64, start: &[u8]) -> Result<Bytes> {
	let row = RowKey::KvRangeWhiteout(ino, start.to_vec());
	prefix_end(&keys::row_prefix(scope, &row)).ok_or(Error::Corrupt)
}

/// Whether `bounds` hold no key, as SlateDB decides before it refuses a scan of them.
fn bounds_empty(bounds: &(Bound<Bytes>, Bound<Bytes>)) -> bool {
	match bounds {
		(Bound::Included(lower), Bound::Included(upper)) => lower > upper,
		(Bound::Included(lower), Bound::Excluded(upper))
		| (Bound::Excluded(lower), Bound::Included(upper))
		| (Bound::Excluded(lower), Bound::Excluded(upper)) => lower >= upper,
		(Bound::Unbounded, _) | (_, Bound::Unbounded) => false,
	}
}

/// The whiteout a start's newest visible version describes, or `None` for a start whose newest
/// visible version is a delete or that the session sees no version of.
fn live_whiteout(start: Option<RowKey>, newest: Option<Option<Row>>) -> Option<RangeWhiteout> {
	match (start, newest) {
		(Some(start), Some(Some(row))) => RangeWhiteout::from_row(&start, &row),
		_ => None,
	}
}

/// Rounds of level pages one overlay scan reads at most before it is refused as too large: each
/// round advances past at least one page of some level, so this bounds the keys a page may skip
/// over whiteouts.
const OVERLAY_ROUNDS: usize = 64;

/// Whether rows of `key`'s kind read through the base chain: the metadata a child volume
/// inherits. Chunks have their own chain walk; claims, orphans and range whiteouts belong to
/// the level that holds them.
fn reads_through_kind(key: &RowKey) -> bool {
	match key {
		RowKey::Format
		| RowKey::Inode(_)
		| RowKey::Dentry(..)
		| RowKey::Link(..)
		| RowKey::Xattr(..)
		| RowKey::Kv(..) => true,
		RowKey::Orphan(_) | RowKey::Chunk(..) | RowKey::KvRangeWhiteout(..) => false,
	}
}

/// A row read through a link as this volume sees it. An inherited file's chunks are read
/// through the same links up to the size the link held, as an explicit fork's copied inode
/// records, so a chunk this volume never wrote reads the parent's bytes, not a hole.
fn inherited(row: Row) -> Row {
	match row {
		Row::Inode(inode) => Row::Inode(depot_fs_core::forks::copy_inode(inode)),
		row => row,
	}
}

/// The rows of a page with their delete markers dropped.
fn present(rows: Vec<(RowKey, u16, Option<Row>)>) -> Vec<(RowKey, u16, Row)> {
	rows.into_iter()
		.filter_map(|(key, claim_log, value)| value.map(|value| (key, claim_log, value)))
		.collect()
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
		RowRange::KvRangeWhiteouts(ino) => {
			keys::KvRangeWhiteoutRowsPrefix::new(hash, name, incarnation, 0, *ino).pack()
		}
		RowRange::KvRangeWhiteoutsAll => {
			keys::KvRangeWhiteoutSpacePrefix::new(hash, name, incarnation, 0).pack()
		}
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
