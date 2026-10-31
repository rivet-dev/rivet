//! Process-wide buffer of pages SQLite has not read: pages fetched ahead of its requests, or
//! pages it committed that its pager may evict. A page leaves the buffer when SQLite reads it,
//! because SQLite's pager then owns it.
//!
//! Every database of one runtime process shares one [`PageBudget`], so the process holds at most
//! the configured number of buffered pages however many databases it opens. Each database's
//! buffers are [`PageBuffer`] handles into that budget.

use std::{
	hash::{Hash, Hasher},
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use lru::LruCache;
use parking_lot::{Mutex, RwLock};
use sqlite_types::PageBytes;

use crate::worker::SqlitePoolMetrics;

/// Expired pages an insert may drop from its shard's cold end.
const EXPIRE_PER_INSERT: usize = 4;

/// Shards of a large budget. A small budget uses one shard, so its eviction order is exact.
const SHARDS: usize = 16;

/// Budgets below this many pages use a single shard.
const MIN_SHARDED_CAPACITY: u64 = 4_096;

/// One process's page budget, shared by every database the process opens.
///
/// Eviction is least recently used across databases within each shard. A page lands in the
/// shard picked by its buffer and page number, so a busy database cannot evict pages only from
/// itself, and an idle one gives up its pages as others insert. The shard locks guard only
/// in-memory maps and are never held across storage I/O.
pub struct PageBudget {
	shards: Box<[Mutex<LruCache<PageKey, Entry>>]>,
	shard_capacity: u64,
	capacity: u64,
	ttl: Option<Duration>,
	used: AtomicU64,
	next_buffer: AtomicU64,
	metrics: Option<Arc<dyn SqlitePoolMetrics>>,
}

impl std::fmt::Debug for PageBudget {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PageBudget")
			.field("capacity", &self.capacity)
			.field("used", &self.used.load(Ordering::Relaxed))
			.field("ttl", &self.ttl)
			.finish()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PageKey {
	buffer: u64,
	page: u32,
}

#[derive(Debug)]
struct Entry {
	page: PageBytes,
	inserted: Instant,
	/// Live pages of the buffer generation that inserted this page.
	owner: Arc<AtomicU64>,
	unread: Option<UnreadPage>,
}

/// One received page's diagnostic identity, allocated only for an enabled TRACE capture.
/// The span carries the native opening and request; `ordinal` distinguishes duplicate pgnos.
#[derive(Debug)]
pub(crate) struct PageTrace {
	span: tracing::Span,
	dispatch: tracing::Dispatch,
	ordinal: usize,
	pgno: u32,
	bytes: usize,
}
impl Drop for PageTrace {
	fn drop(&mut self) {
		// The registry closes parent references through the ambient subscriber.
		// Retained contexts can be released by another worker or request collector.
		tracing::dispatcher::with_default(&self.dispatch, || {
			drop(std::mem::replace(&mut self.span, tracing::Span::none()));
		});
	}
}

impl PageTrace {
	pub(crate) fn capture(ordinal: usize, pgno: u32, bytes: usize) -> Option<Box<Self>> {
		tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE).then(|| {
			Box::new(Self {
				span: tracing::Span::current(),
				dispatch: tracing::dispatcher::get_default(Clone::clone),
				ordinal,
				pgno,
				bytes,
			})
		})
	}
	pub(crate) fn disposition(&self, disposition: &'static str) {
		tracing::dispatcher::with_default(&self.dispatch, || {
			self.span.in_scope(|| {
				tracing::trace!(target: "sqlite_prefetch::trace", event = "page_disposition",
                ordinal = self.ordinal, pgno = self.pgno, bytes = self.bytes, disposition);
			})
		});
	}
	fn buffered(&self, generation: &Arc<UnreadTracker>) {
		// Reuse the buffer's existing monotonic identity. Allocator addresses can be
		// recycled while the same VFS opening continues issuing requests.
		tracing::dispatcher::with_default(&self.dispatch, || {
			self.span.in_scope(|| {
				tracing::trace!(target: "sqlite_prefetch::trace", event = "page_buffered",
                buffer_generation = generation.id,
                ordinal = self.ordinal, pgno = self.pgno, bytes = self.bytes);
			})
		});
	}
}

#[derive(Debug, Clone)]
struct CloseTrace {
	span: tracing::Span,
	dispatch: tracing::Dispatch,
}
impl Drop for CloseTrace {
	fn drop(&mut self) {
		// The registry closes parent references through the ambient subscriber.
		// Retained contexts can be released by another worker or request collector.
		tracing::dispatcher::with_default(&self.dispatch, || {
			drop(std::mem::replace(&mut self.span, tracing::Span::none()));
		});
	}
}

/// Counts unread removals once, including immediate release of a whole generation.
#[derive(Debug)]
struct UnreadTracker {
	id: u64,
	state: Mutex<UnreadState>,
	total: Arc<AtomicU64>,
}
#[derive(Debug)]
struct UnreadState {
	count: Option<u64>,
	trace: Option<CloseTrace>,
	traced_unread: u64,
}
impl UnreadTracker {
	fn new(total: Arc<AtomicU64>, id: u64) -> Arc<Self> {
		Arc::new(Self {
			id,
			state: Mutex::new(UnreadState {
				count: Some(0),
				trace: None,
				traced_unread: 0,
			}),
			total,
		})
	}
	fn page(self: &Arc<Self>, trace: Option<Box<PageTrace>>) -> UnreadPage {
		let mut state = self.state.lock();
		if let Some(count) = state.count.as_mut() {
			*count += 1;
			if let Some(trace) = &trace {
				state.traced_unread += 1;
				if state.trace.is_none() {
					state.trace = Some(CloseTrace {
						span: trace.span.clone(),
						dispatch: trace.dispatch.clone(),
					});
				}
				trace.buffered(self);
			}
		} else {
			self.total.fetch_add(1, Ordering::Relaxed);
			if let Some(trace) = &trace {
				trace.disposition("unused");
			}
		}
		UnreadPage {
			owner: self.clone(),
			consumed: false,
			trace,
		}
	}
	fn release(&self, consumed: bool, trace: Option<&PageTrace>) {
		let mut state = self.state.lock();
		if let Some(count) = state.count.as_mut() {
			*count -= 1;
			if !consumed {
				self.total.fetch_add(1, Ordering::Relaxed);
			}
			if let Some(trace) = trace {
				state.traced_unread -= 1;
				trace.disposition(if consumed { "consumed" } else { "unused" });
			}
		}
	}
	fn close(&self) {
		let mut state = self.state.lock();
		if let Some(count) = state.count.take() {
			self.total.fetch_add(count, Ordering::Relaxed);
			if let Some(trace) = state.trace.take() {
				// O(1): outstanding page_buffered deliveries for this generation become
				// unused here. Ghost-entry drops after this event emit no second outcome.
				tracing::dispatcher::with_default(&trace.dispatch, || {
					trace.span.in_scope(|| {
						tracing::trace!(target: "sqlite_prefetch::trace", event = "buffer_closed",
                        buffer_generation = self.id, unread_pages = count, unread_traced_pages = state.traced_unread);
					})
				});
			}
		}
	}
}
#[derive(Debug)]
struct UnreadPage {
	owner: Arc<UnreadTracker>,
	consumed: bool,
	trace: Option<Box<PageTrace>>,
}
impl UnreadPage {
	fn consume(&mut self) {
		if !self.consumed {
			self.owner.release(true, self.trace.as_deref());
			self.consumed = true;
		}
	}
}
impl Drop for UnreadPage {
	fn drop(&mut self) {
		if !self.consumed {
			self.owner.release(false, self.trace.as_deref());
		}
	}
}

impl PageBudget {
	/// A budget of `capacity` pages shared by every buffer made from it. Zero disables
	/// buffering. Pages older than `ttl_ms` expire; zero keeps them until evicted.
	pub fn new(
		capacity: u64,
		ttl_ms: u64,
		metrics: Option<Arc<dyn SqlitePoolMetrics>>,
	) -> Arc<Self> {
		let shard_count = if capacity >= MIN_SHARDED_CAPACITY {
			SHARDS
		} else {
			1
		};
		// Allocate as pages arrive, rather than reserving the configured maximum up front.
		let shards = (0..shard_count)
			.map(|_| Mutex::new(LruCache::unbounded()))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		if let Some(metrics) = &metrics {
			metrics.set_page_buffer_capacity_pages(capacity);
			metrics.set_page_buffer_pages(0);
		}
		Arc::new(Self {
			shards,
			shard_capacity: capacity.div_ceil(shard_count as u64),
			capacity,
			ttl: (ttl_ms > 0).then(|| Duration::from_millis(ttl_ms)),
			used: AtomicU64::new(0),
			next_buffer: AtomicU64::new(1),
			metrics,
		})
	}

	/// Configured page capacity of the whole process.
	pub fn capacity_pages(&self) -> u64 {
		self.capacity
	}

	/// Pages held across every database, including expired pages awaiting eviction and pages
	/// of released buffers that the LRU has not reached yet.
	pub fn used_pages(&self) -> u64 {
		self.used.load(Ordering::Relaxed)
	}

	fn shard(&self, key: &PageKey) -> &Mutex<LruCache<PageKey, Entry>> {
		if self.shards.len() == 1 {
			return &self.shards[0];
		}
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		key.hash(&mut hasher);
		&self.shards[hasher.finish() as usize % self.shards.len()]
	}

	fn allocate_buffer(&self) -> u64 {
		self.next_buffer.fetch_add(1, Ordering::Relaxed)
	}

	/// Accounts for one page leaving the budget.
	fn released(&self, entry: &Entry) {
		entry.owner.fetch_sub(1, Ordering::Relaxed);
		let used = self.used.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
		if let Some(metrics) = &self.metrics {
			metrics.set_page_buffer_pages(used);
		}
	}

	fn added(&self) {
		let used = self.used.fetch_add(1, Ordering::Relaxed).saturating_add(1);
		if let Some(metrics) = &self.metrics {
			metrics.set_page_buffer_pages(used);
		}
	}

	fn expired(&self, entry: &Entry, now: Instant) -> bool {
		self.ttl
			.is_some_and(|ttl| now.duration_since(entry.inserted) >= ttl)
	}
}

/// One database buffer in the process budget. Clones share pages with asynchronous prefetch
/// completion.
#[derive(Clone, Debug)]
pub(crate) struct PageBuffer {
	inner: Arc<BufferInner>,
}

#[derive(Debug)]
struct BufferInner {
	budget: Arc<PageBudget>,
	/// The current generation: invalidating everything moves to a fresh id, so older pages
	/// become unreachable and age out of the budget without a scan.
	generation: RwLock<Generation>,
}

#[derive(Debug, Clone)]
struct Generation {
	id: u64,
	live: Arc<AtomicU64>,
	unread: Arc<UnreadTracker>,
}

impl PageBuffer {
	pub(crate) fn new(budget: &Arc<PageBudget>) -> Self {
		Self::with_unused_counter(budget, Arc::new(AtomicU64::new(0)))
	}

	pub(crate) fn with_unused_counter(budget: &Arc<PageBudget>, total: Arc<AtomicU64>) -> Self {
		let id = budget.allocate_buffer();
		Self {
			inner: Arc::new(BufferInner {
				budget: budget.clone(),
				generation: RwLock::new(Generation {
					id,
					live: Arc::new(AtomicU64::new(0)),
					unread: UnreadTracker::new(total, id),
				}),
			}),
		}
	}

	fn generation(&self) -> Generation {
		self.inner.generation.read().clone()
	}

	fn key(&self, generation: &Generation, page: u32) -> PageKey {
		PageKey {
			buffer: generation.id,
			page,
		}
	}

	pub(crate) fn get(&self, page: &u32) -> Option<PageBytes> {
		self.get_at(page, cache_now())
	}

	fn get_at(&self, page: &u32, now: Instant) -> Option<PageBytes> {
		self.lookup_at(page, now, true)
	}

	fn lookup_at(&self, page: &u32, now: Instant, consume: bool) -> Option<PageBytes> {
		let budget = &self.inner.budget;
		let generation = self.generation();
		let key = self.key(&generation, *page);
		let mut shard = budget.shard(&key).lock();
		let entry = shard.get_mut(&key)?;
		if budget.expired(entry, now) {
			if let Some(entry) = shard.pop(&key) {
				budget.released(&entry);
			}
			None
		} else {
			if consume && let Some(unread) = entry.unread.as_mut() {
				unread.consume();
			}
			Some(entry.page.clone())
		}
	}

	/// Removes and returns a live page, handing it to the caller. An expired page is dropped.
	pub(crate) fn take(&self, page: &u32) -> Option<PageBytes> {
		self.take_at(page, cache_now())
	}

	fn take_at(&self, page: &u32, now: Instant) -> Option<PageBytes> {
		self.take_status_at(page, now).map(|(bytes, _)| bytes)
	}
	pub(crate) fn take_with_status(&self, page: &u32) -> Option<(PageBytes, bool)> {
		self.take_status_at(page, cache_now())
	}
	fn take_status_at(&self, page: &u32, now: Instant) -> Option<(PageBytes, bool)> {
		let budget = &self.inner.budget;
		let generation = self.generation();
		let key = self.key(&generation, *page);
		let mut entry = budget.shard(&key).lock().pop(&key)?;
		budget.released(&entry);
		if budget.expired(&entry, now) {
			None
		} else {
			let was_unread = entry.unread.as_ref().is_some_and(|unread| !unread.consumed);
			if let Some(unread) = entry.unread.as_mut() {
				unread.consume();
			}
			Some((entry.page, was_unread))
		}
	}

	

	pub(crate) fn insert(&self, page: u32, bytes: PageBytes) {
		self.insert_at(page, bytes, cache_now());
	}

	pub(crate) fn insert_prefetched(&self, page: u32, bytes: PageBytes) {
		self.insert_traced_at(page, bytes, cache_now(), true, None);
	}
	pub(crate) fn insert_prefetched_traced(
		&self,
		page: u32,
		bytes: PageBytes,
		trace: Option<Box<PageTrace>>,
	) {
		self.insert_traced_at(page, bytes, cache_now(), true, trace);
	}
	fn insert_at(&self, page: u32, bytes: PageBytes, now: Instant) {
		self.insert_tracked_at(page, bytes, now, false);
	}
	fn insert_tracked_at(&self, page: u32, bytes: PageBytes, now: Instant, track: bool) {
		self.insert_traced_at(page, bytes, now, track, None);
	}
	fn insert_traced_at(
		&self,
		page: u32,
		bytes: PageBytes,
		now: Instant,
		track: bool,
		trace: Option<Box<PageTrace>>,
	) {
		let budget = &self.inner.budget;
		if budget.capacity == 0 {
			if let Some(trace) = &trace {
				trace.disposition("unused");
			}
			if track {
				self.generation()
					.unread
					.total
					.fetch_add(1, Ordering::Relaxed);
			}
			return;
		}
		let generation = self.generation();
		let key = self.key(&generation, page);
		let mut shard = budget.shard(&key).lock();
		// Busy shards shed stale pages at a bounded cost per insert.
		expire_tail(budget, &mut shard, now, EXPIRE_PER_INSERT);
		generation.live.fetch_add(1, Ordering::Relaxed);
		budget.added();
		let entry = Entry {
			page: bytes,
			inserted: now,
			owner: generation.live,
			unread: track.then(|| generation.unread.page(trace)),
		};
		if let Some((_, replaced)) = shard.push(key, entry) {
			budget.released(&replaced);
		}
		while shard.len() as u64 > budget.shard_capacity {
			let Some((_, evicted)) = shard.pop_lru() else {
				break;
			};
			budget.released(&evicted);
		}
	}

	pub(crate) fn invalidate(&self, page: &u32) {
		let budget = &self.inner.budget;
		let generation = self.generation();
		let key = self.key(&generation, *page);
		let removed = budget.shard(&key).lock().pop(&key);
		if let Some(entry) = removed {
			budget.released(&entry);
		}
	}

	/// Drops every page of this buffer. Later reads miss at once; the dropped pages stay in the
	/// budget, unreachable, until the LRU or their expiry reaches them.
	pub(crate) fn invalidate_all(&self) {
		let budget = &self.inner.budget;
		let mut generation = self.inner.generation.write();
		generation.unread.close();
		let id = budget.allocate_buffer();
		*generation = Generation {
			id,
			live: Arc::new(AtomicU64::new(0)),
			unread: UnreadTracker::new(generation.unread.total.clone(), id),
		};
	}

	/// Live pages of this buffer, including lazily expired pages awaiting eviction or access.
	pub(crate) fn entry_count(&self) -> u64 {
		self.generation().live.load(Ordering::Relaxed)
	}

	pub(crate) fn weighted_size(&self) -> u64 {
		self.entry_count()
	}
}

/// Pops expired pages from the least recently used end of a shard, stopping at the first live
/// page or after `limit` pages. A stale page that a read moved to the front stays until a read
/// finds it.
fn expire_tail(
	budget: &PageBudget,
	shard: &mut LruCache<PageKey, Entry>,
	now: Instant,
	limit: usize,
) -> usize {
	if budget.ttl.is_none() {
		return 0;
	}
	let mut expired = 0;
	while expired < limit {
		let Some((_, entry)) = shard.peek_lru() else {
			break;
		};
		if !budget.expired(entry, now) {
			break;
		}
		if let Some((_, entry)) = shard.pop_lru() {
			budget.released(&entry);
		}
		expired += 1;
	}
	expired
}

// This actor-side SQLite adapter runs on synchronous native worker threads. Cache
// expiration only evicts optional read copies; it cannot affect storage durability.
#[allow(clippy::disallowed_methods)]
fn cache_now() -> Instant {
	Instant::now()
}



impl Drop for BufferInner {
	fn drop(&mut self) {
		self.generation.get_mut().unread.close();
	}
}
