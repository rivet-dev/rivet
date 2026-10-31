//! Process-wide SQLite page cache, installed through `SQLITE_CONFIG_PCACHE2`.
//!
//! SQLite's default page cache gives every connection its own `cache_size` budget, so a process
//! with many databases holds up to the sum of all of them while a busy database is capped at its
//! own share. This cache replaces SQLite's page cache for every connection in the process: one
//! byte budget covers the pages of every purgeable cache, so a busy database takes pages from
//! idle ones and the process total stays bounded.
//!
//! The cache follows SQLite's page cache contract:
//! - A page returned by `xFetch` is pinned and stays at its address until `xUnpin`.
//! - An unpinned page of a purgeable cache may be discarded at any time, because its pager keeps
//!   no reference to it (SQLite never unpins a dirty page). Discarding one from another
//!   connection's cache is therefore safe.
//! - A non-purgeable cache (an in-memory or temporary database, which `temp_store = MEMORY`
//!   makes of every temporary b-tree) keeps every page until `xUnpin` with `discard`,
//!   `xTruncate` or `xDestroy`, because those pages are the database itself. Its pages are
//!   counted apart and never charged to the budget, so one connection's large sort or `VACUUM`
//!   cannot push every other connection's pages out.
//! - `xFetch` with `createFlag` 1 may refuse when the budget is full of pinned pages, which tells
//!   the pager to spill; with `createFlag` 2 it allocates past the budget rather than fail, and
//!   the overshoot is returned when pages are unpinned. Purgeable bytes can therefore exceed the
//!   budget by the pages SQLite still pins after a refusal it cannot spill around: pages held by
//!   open cursors, and dirty pages when no spill is possible.
//! - A new page, including its extra area, starts zeroed, because SQLite's pager reads its header
//!   from the extra area to tell a new page from a recycled one.
//!
//! Every page access takes only the lock of the shard that owns its connection's cache, so
//! connections on different shards never wait for each other. Each shard keeps its own least
//! recently used order; eviction walks the shards from a rotating cursor, which approximates one
//! process-wide order without a global lock. Pages are allocated and freed outside every lock.
//! Every call on one connection's cache runs under that connection's database mutex, or the
//! shared b-tree's mutex in shared-cache mode, so a miss can drop the shard lock while it
//! allocates without racing another call on the same cache. A miss still checks the key again
//! when it re-locks, so a page it finds there is returned instead of overwritten.

use std::{
	alloc::{self, Layout},
	cell::UnsafeCell,
	collections::HashMap,
	ffi::{c_int, c_uint, c_void},
	ptr::{self, NonNull},
	sync::{
		Arc, OnceLock,
		atomic::{AtomicPtr, AtomicU64, AtomicUsize, Ordering},
	},
};

use libsqlite3_sys::{
	SQLITE_CONFIG_PCACHE2, SQLITE_OK, sqlite3_config, sqlite3_pcache, sqlite3_pcache_methods2,
	sqlite3_pcache_page,
};
use lru::LruCache;
use parking_lot::Mutex;

use crate::worker::SqlitePoolMetrics;

/// Shards of the cache. Connections are spread over them by creation order.
const SHARDS: usize = 16;

/// The installed cache. SQLite's page cache configuration is process-global C state that holds
/// a pointer to this object for the life of the process, so it cannot be owned by a pool or a
/// database; this static is the one owner that matches that lifetime. It is published before
/// `sqlite3_config` runs, so `xCreate` always finds it once SQLite can call it.
static PROCESS_PAGER_CACHE: AtomicPtr<PagerCache> = AtomicPtr::new(ptr::null_mut());

/// Whether `install` already ran in this process, and whether it succeeded.
static INSTALLED: OnceLock<bool> = OnceLock::new();

/// Installs the process-wide page cache with `capacity_bytes`, once per process.
///
/// It must run before anything else in the process uses SQLite, because SQLite accepts a page
/// cache only before it initializes and `sqlite3_config` itself is not thread safe. Production
/// calls it from the first `SqlitePool::new`, which runs at startup before any database opens.
/// If SQLite is already initialized (another library opened a database first), the install fails
/// and connections keep SQLite's per-connection cache. Returns whether the process-wide cache is
/// active. Later calls only attach `metrics` if none are attached yet.
pub fn install(capacity_bytes: u64, metrics: Option<Arc<dyn SqlitePoolMetrics>>) -> bool {
	let installed = *INSTALLED.get_or_init(|| {
		if capacity_bytes == 0 {
			return false;
		}
		// SQLite keeps `pArg` and calls into this cache until the process exits, and offers no
		// point where the pointer is released, so the cache lives as long as the process. This
		// runs once per process, so the leak is bounded to one cache.
		let cache: &'static PagerCache = Box::leak(Box::new(PagerCache::new(capacity_bytes)));
		PROCESS_PAGER_CACHE.store(cache as *const PagerCache as *mut PagerCache, Ordering::Release);
		let methods = sqlite3_pcache_methods2 {
			iVersion: 1,
			pArg: cache as *const PagerCache as *mut c_void,
			xInit: Some(x_init),
			xShutdown: Some(x_shutdown),
			xCreate: Some(x_create),
			xCachesize: Some(x_cachesize),
			xPagecount: Some(x_pagecount),
			xFetch: Some(x_fetch),
			xUnpin: Some(x_unpin),
			xRekey: Some(x_rekey),
			xTruncate: Some(x_truncate),
			xDestroy: Some(x_destroy),
			xShrink: Some(x_shrink),
		};
		// SAFETY: SQLite copies the methods struct; `pArg` points at a cache leaked for the
		// life of the process, once, so every later callback finds it alive.
		let rc = unsafe { sqlite3_config(SQLITE_CONFIG_PCACHE2, &methods as *const _) };
		if rc != SQLITE_OK {
			PROCESS_PAGER_CACHE.store(ptr::null_mut(), Ordering::Release);
			tracing::warn!(
				rc,
				"sqlite was initialized before the process-wide page cache could be installed; connections keep per-connection page caches"
			);
			return false;
		}
		true
	});
	if let (true, Some(metrics)) = (installed, metrics) {
		attach_metrics(metrics);
	}
	installed
}

/// The installed cache, if any, for tests and metrics.
pub fn installed() -> Option<&'static PagerCache> {
	let cache = PROCESS_PAGER_CACHE.load(Ordering::Acquire);
	// SAFETY: a non-null pointer is a cache leaked for the life of the process.
	unsafe { cache.as_ref() }
}

/// Reports the installed cache's gauges through `metrics`, unless another pool already attached
/// its metrics. The cache is process-wide, so whichever pool carries metrics reports it,
/// regardless of which pool installed it.
pub fn attach_metrics(metrics: Arc<dyn SqlitePoolMetrics>) {
	if let Some(cache) = installed()
		&& cache.metrics.set(metrics).is_ok()
	{
		cache.report_capacity();
		cache.report();
	}
}

/// Every page of every connection's cache.
pub struct PagerCache {
	shards: Box<[Mutex<Shard>]>,
	capacity_bytes: u64,
	/// Bytes of purgeable pages, the only bytes the budget bounds.
	purgeable_bytes: AtomicU64,
	/// Bytes of non-purgeable pages, which are the in-memory databases themselves.
	unbudgeted_bytes: AtomicU64,
	pages: AtomicU64,
	/// `createFlag` 1 fetches refused so SQLite would spill, for tests and diagnostics.
	refusals: AtomicU64,
	/// Pages discarded to make room, for tests and diagnostics.
	evictions: AtomicU64,
	next_cache: AtomicUsize,
	evict_cursor: AtomicUsize,
	metrics: OnceLock<Arc<dyn SqlitePoolMetrics>>,
}

/// One shard's unpinned pages of purgeable caches, least recently unpinned first. A key is the
/// owning cache's handle address and the page number.
struct Shard {
	lru: LruCache<(usize, u32), ()>,
}

/// One connection's cache, handed to SQLite as its `sqlite3_pcache` pointer.
struct CacheHandle {
	cache: &'static PagerCache,
	shard: usize,
	/// Reached only while holding `cache.shards[shard]`.
	state: UnsafeCell<CacheState>,
}

struct CacheState {
	/// Allocation of one page: the slot header, then the page and extra areas.
	layout: Layout,
	/// Offset of the page area within an allocation.
	buf_offset: usize,
	/// Bytes one page charges: its allocation size.
	slot_bytes: u64,
	page_size: usize,
	purgeable: bool,
	pages: HashMap<u32, NonNull<Slot>>,
	/// An upper bound on every key ever held, so `xTruncate` above it skips the scan.
	max_key: u32,
}

/// One page's header, at the start of its allocation. `page` is first so a
/// `*mut sqlite3_pcache_page` handed to SQLite is the slot's address.
#[repr(C)]
struct Slot {
	page: sqlite3_pcache_page,
	key: u32,
	pinned: bool,
}

/// Pages freed by a call, released after its lock is dropped.
type Freed = Vec<(NonNull<Slot>, Layout)>;

impl PagerCache {
	fn new(capacity_bytes: u64) -> Self {
		Self {
			shards: (0..SHARDS)
				.map(|_| {
					Mutex::new(Shard {
						lru: LruCache::unbounded(),
					})
				})
				.collect(),
			capacity_bytes,
			purgeable_bytes: AtomicU64::new(0),
			unbudgeted_bytes: AtomicU64::new(0),
			pages: AtomicU64::new(0),
			refusals: AtomicU64::new(0),
			evictions: AtomicU64::new(0),
			next_cache: AtomicUsize::new(0),
			evict_cursor: AtomicUsize::new(0),
			metrics: OnceLock::new(),
		}
	}

	/// Bytes of purgeable pages across every connection, the bytes the budget bounds.
	pub fn bytes(&self) -> u64 {
		self.purgeable_bytes.load(Ordering::Relaxed)
	}

	/// Bytes of non-purgeable pages (in-memory and temporary databases), outside the budget.
	pub fn unbudgeted_bytes(&self) -> u64 {
		self.unbudgeted_bytes.load(Ordering::Relaxed)
	}

	/// Pages the cache holds across every connection.
	pub fn pages(&self) -> u64 {
		self.pages.load(Ordering::Relaxed)
	}

	/// The configured budget.
	pub fn capacity_bytes(&self) -> u64 {
		self.capacity_bytes
	}

	/// `createFlag` 1 fetches refused because the budget was full of pinned pages.
	pub fn refusals(&self) -> u64 {
		self.refusals.load(Ordering::Relaxed)
	}

	/// Pages discarded to make room for others.
	pub fn evictions(&self) -> u64 {
		self.evictions.load(Ordering::Relaxed)
	}

	fn report_capacity(&self) {
		if let Some(metrics) = self.metrics.get() {
			metrics.set_pager_cache_capacity_bytes(self.capacity_bytes);
		}
	}

	/// Publishes the counts. Called only when a page is allocated or freed, never on a hit.
	fn report(&self) {
		if let Some(metrics) = self.metrics.get() {
			metrics.set_pager_cache_bytes(self.bytes());
			metrics.set_pager_cache_unbudgeted_bytes(self.unbudgeted_bytes());
			metrics.set_pager_cache_pages(self.pages());
		}
	}

	/// Frees pages collected under a lock, after the lock is dropped.
	fn release(&self, freed: Freed) {
		if freed.is_empty() {
			return;
		}
		for (slot, layout) in freed {
			// SAFETY: every slot was allocated with `layout` in `allocate`, removed from its
			// cache's map and LRU under the lock, and is freed exactly once here.
			unsafe { alloc::dealloc(slot.as_ptr() as *mut u8, layout) };
		}
		self.report();
	}

	/// Charges or credits one page of `state` to the counters.
	fn account(&self, state: &CacheState, added: bool) {
		let counter = if state.purgeable {
			&self.purgeable_bytes
		} else {
			&self.unbudgeted_bytes
		};
		if added {
			counter.fetch_add(state.slot_bytes, Ordering::Relaxed);
			self.pages.fetch_add(1, Ordering::Relaxed);
		} else {
			counter.fetch_sub(state.slot_bytes, Ordering::Relaxed);
			self.pages.fetch_sub(1, Ordering::Relaxed);
		}
	}

	/// Reserves `bytes` of the budget for a purgeable page, evicting unpinned pages until it
	/// fits. Returns false when nothing is left to evict and the budget is still full.
	fn reserve(&self, bytes: u64) -> bool {
		loop {
			let current = self.purgeable_bytes.load(Ordering::Relaxed);
			if current.saturating_add(bytes) <= self.capacity_bytes {
				if self
					.purgeable_bytes
					.compare_exchange_weak(
						current,
						current + bytes,
						Ordering::Relaxed,
						Ordering::Relaxed,
					)
					.is_ok()
				{
					return true;
				}
				continue;
			}
			if !self.evict_one() {
				return false;
			}
		}
	}

	/// Evicts unpinned pages until the budget is no longer exceeded, or none are left.
	fn shed_overshoot(&self) {
		while self.bytes() > self.capacity_bytes {
			if !self.evict_one() {
				return;
			}
		}
	}

	/// Discards one least recently unpinned page, trying each shard once from a rotating
	/// cursor. Takes one shard lock at a time, never two. Returns false when no shard has an
	/// unpinned page.
	fn evict_one(&self) -> bool {
		for _ in 0..SHARDS {
			let shard_index = self.evict_cursor.fetch_add(1, Ordering::Relaxed) % SHARDS;
			let mut shard = self.shards[shard_index].lock();
			let Some(((handle_addr, key), ())) = shard.lru.pop_lru() else {
				continue;
			};
			// SAFETY: a key in the LRU names a live handle of this shard; `x_destroy` removes
			// every key of a handle under this lock before the handle is freed, and its state
			// is reached under this lock.
			let state = unsafe { &mut *(*(handle_addr as *const CacheHandle)).state.get() };
			let freed = state.pages.remove(&key).map(|slot| (slot, state.layout));
			if freed.is_some() {
				self.account(state, false);
				self.evictions.fetch_add(1, Ordering::Relaxed);
			}
			drop(shard);
			self.release(freed.into_iter().collect());
			return true;
		}
		false
	}
}

/// Removes `key` from `state` and the shard's LRU, returning the page to free.
fn take_page(
	cache: &PagerCache,
	shard: &mut Shard,
	handle_addr: usize,
	state: &mut CacheState,
	key: u32,
	freed: &mut Freed,
) {
	if let Some(slot) = state.pages.remove(&key) {
		shard.lru.pop(&(handle_addr, key));
		cache.account(state, false);
		freed.push((slot, state.layout));
	}
}

/// Allocates a pinned, zeroed page for `key` of a cache with `state`'s layout, without holding
/// any lock. Returns `None` when memory is exhausted, which SQLite reports as `SQLITE_NOMEM`.
fn allocate(
	layout: Layout,
	buf_offset: usize,
	page_size: usize,
	key: u32,
) -> Option<NonNull<Slot>> {
	// SAFETY: `layout` has a non-zero size, because it includes the slot header.
	let base = NonNull::new(unsafe { alloc::alloc_zeroed(layout) })?;
	let slot = base.cast::<Slot>();
	// SAFETY: the allocation holds the header at its start and `page_size` plus the extra area at
	// `buf_offset`, all derived from `base`, so the page pointers stay valid until it is freed.
	unsafe {
		let buf = base.as_ptr().add(buf_offset);
		ptr::write(
			slot.as_ptr(),
			Slot {
				page: sqlite3_pcache_page {
					pBuf: buf as *mut c_void,
					pExtra: buf.add(page_size) as *mut c_void,
				},
				key,
				pinned: true,
			},
		);
	}
	Some(slot)
}

/// The handle SQLite passes back for a cache.
///
/// # Safety
/// `cache` must be a pointer returned by `x_create` and not yet passed to `x_destroy`.
unsafe fn handle<'a>(cache: *mut sqlite3_pcache) -> &'a CacheHandle {
	unsafe { &*(cache as *const CacheHandle) }
}

unsafe extern "C" fn x_init(_arg: *mut c_void) -> c_int {
	SQLITE_OK
}

unsafe extern "C" fn x_shutdown(_arg: *mut c_void) {}

unsafe extern "C" fn x_create(
	page_size: c_int,
	extra_size: c_int,
	purgeable: c_int,
) -> *mut sqlite3_pcache {
	// `xCreate` receives no `pArg`, so it reaches the installed cache through the process owner.
	match installed() {
		Some(cache) => create(cache, page_size, extra_size, purgeable),
		None => ptr::null_mut(),
	}
}

/// Creates one connection's cache inside `cache`. Returns null if the page layout is invalid,
/// which SQLite reports as `SQLITE_NOMEM`.
fn create(
	cache: &'static PagerCache,
	page_size: c_int,
	extra_size: c_int,
	purgeable: c_int,
) -> *mut sqlite3_pcache {
	let page_size = page_size.max(0) as usize;
	let extra_size = extra_size.max(0) as usize;
	let Ok(area) = Layout::from_size_align(page_size + extra_size, 8) else {
		return ptr::null_mut();
	};
	let Ok((layout, buf_offset)) = Layout::new::<Slot>().extend(area) else {
		return ptr::null_mut();
	};
	let layout = layout.pad_to_align();
	let shard = cache.next_cache.fetch_add(1, Ordering::Relaxed) % SHARDS;
	let handle = Box::new(CacheHandle {
		cache,
		shard,
		state: UnsafeCell::new(CacheState {
			layout,
			buf_offset,
			slot_bytes: layout.size() as u64,
			page_size,
			purgeable: purgeable != 0,
			pages: HashMap::new(),
			max_key: 0,
		}),
	});
	Box::into_raw(handle) as *mut sqlite3_pcache
}

/// With this cache, `cache_size` reaches it only through this callback, and the process budget
/// replaces each connection's `cache_size`, so the suggestion is ignored. SQLite decides when to
/// spill from `createFlag` 1 refusals and `cache_spill`, not from `cache_size`.
unsafe extern "C" fn x_cachesize(_cache: *mut sqlite3_pcache, _pages: c_int) {}

unsafe extern "C" fn x_pagecount(cache: *mut sqlite3_pcache) -> c_int {
	// SAFETY: SQLite passes a live cache.
	let handle = unsafe { handle(cache) };
	let _shard = handle.cache.shards[handle.shard].lock();
	// SAFETY: the state is reached under its shard's lock.
	let state = unsafe { &*handle.state.get() };
	state.pages.len().min(c_int::MAX as usize) as c_int
}

unsafe extern "C" fn x_fetch(
	cache: *mut sqlite3_pcache,
	key: c_uint,
	create_flag: c_int,
) -> *mut sqlite3_pcache_page {
	// SAFETY: SQLite passes a live cache.
	let handle = unsafe { handle(cache) };
	let pager = handle.cache;
	let handle_addr = handle as *const CacheHandle as usize;

	// The hit path: one shard lock, one map lookup, and an LRU removal for an unpinned page.
	let (layout, buf_offset, page_size, slot_bytes, purgeable) = {
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		if let Some(slot) = state.pages.get(&key).copied() {
			// SAFETY: a slot in the map is alive and reached only under the lock.
			let slot_ref = unsafe { &mut *slot.as_ptr() };
			if !slot_ref.pinned {
				slot_ref.pinned = true;
				shard.lru.pop(&(handle_addr, key));
			}
			return slot.as_ptr() as *mut sqlite3_pcache_page;
		}
		if create_flag == 0 {
			return ptr::null_mut();
		}
		(
			state.layout,
			state.buf_offset,
			state.page_size,
			state.slot_bytes,
			state.purgeable,
		)
	};

	// A miss: reserve the budget and allocate with no lock held. Calls on one cache run under its
	// connection's database mutex (or the shared b-tree's mutex), so no other call on this cache
	// races this one; eviction from other threads only removes unpinned pages.
	if purgeable && !pager.reserve(slot_bytes) {
		// A purgeable cache refuses a page past the budget when SQLite can spill instead.
		if create_flag == 1 {
			pager.refusals.fetch_add(1, Ordering::Relaxed);
			return ptr::null_mut();
		}
		pager
			.purgeable_bytes
			.fetch_add(slot_bytes, Ordering::Relaxed);
	}
	let Some(slot) = allocate(layout, buf_offset, page_size, key) else {
		if purgeable {
			pager
				.purgeable_bytes
				.fetch_sub(slot_bytes, Ordering::Relaxed);
		}
		return ptr::null_mut();
	};

	{
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		if let Some(existing) = state.pages.get(&key).copied() {
			// The key appeared while the lock was dropped. Keep the existing page, pinned and
			// out of the LRU, and give back the page just allocated and its charge.
			// SAFETY: a slot in the map is alive and reached only under the lock.
			let existing_ref = unsafe { &mut *existing.as_ptr() };
			if !existing_ref.pinned {
				existing_ref.pinned = true;
				shard.lru.pop(&(handle_addr, key));
			}
			drop(shard);
			if purgeable {
				pager
					.purgeable_bytes
					.fetch_sub(slot_bytes, Ordering::Relaxed);
			}
			// SAFETY: allocated with `layout` above and never published.
			unsafe { alloc::dealloc(slot.as_ptr() as *mut u8, layout) };
			return existing.as_ptr() as *mut sqlite3_pcache_page;
		}
		state.pages.insert(key, slot);
		state.max_key = state.max_key.max(key);
	}
	if !purgeable {
		pager
			.unbudgeted_bytes
			.fetch_add(slot_bytes, Ordering::Relaxed);
	}
	pager.pages.fetch_add(1, Ordering::Relaxed);
	pager.report();
	slot.as_ptr() as *mut sqlite3_pcache_page
}

unsafe extern "C" fn x_unpin(
	cache: *mut sqlite3_pcache,
	page: *mut sqlite3_pcache_page,
	discard: c_int,
) {
	// SAFETY: SQLite passes a live cache and a page this cache returned.
	let handle = unsafe { handle(cache) };
	let pager = handle.cache;
	let handle_addr = handle as *const CacheHandle as usize;
	let mut freed = Freed::new();
	{
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		// SAFETY: `page` is the first field of a live `Slot`.
		let slot = unsafe { &mut *(page as *mut Slot) };
		if discard != 0 {
			take_page(pager, &mut shard, handle_addr, state, slot.key, &mut freed);
		} else {
			slot.pinned = false;
			if state.purgeable {
				shard.lru.put((handle_addr, slot.key), ());
			}
		}
	}
	pager.release(freed);
	// Pages allocated past the budget while pinned are returned here.
	pager.shed_overshoot();
}

unsafe extern "C" fn x_rekey(
	cache: *mut sqlite3_pcache,
	_page: *mut sqlite3_pcache_page,
	old_key: c_uint,
	new_key: c_uint,
) {
	if old_key == new_key {
		return;
	}
	// SAFETY: SQLite passes a live cache and a page this cache returned.
	let handle = unsafe { handle(cache) };
	let pager = handle.cache;
	let handle_addr = handle as *const CacheHandle as usize;
	let mut freed = Freed::new();
	{
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		// Any page already at `new_key` is unpinned by contract and is discarded.
		take_page(pager, &mut shard, handle_addr, state, new_key, &mut freed);
		if let Some(slot) = state.pages.remove(&old_key) {
			// SAFETY: the slot is alive and reached only under the lock.
			let pinned = unsafe {
				let slot = &mut *slot.as_ptr();
				slot.key = new_key;
				slot.pinned
			};
			state.pages.insert(new_key, slot);
			state.max_key = state.max_key.max(new_key);
			if !pinned && shard.lru.pop(&(handle_addr, old_key)).is_some() {
				shard.lru.put((handle_addr, new_key), ());
			}
		}
	}
	pager.release(freed);
}

unsafe extern "C" fn x_truncate(cache: *mut sqlite3_pcache, limit: c_uint) {
	// SAFETY: SQLite passes a live cache.
	let handle = unsafe { handle(cache) };
	let pager = handle.cache;
	let handle_addr = handle as *const CacheHandle as usize;
	let mut freed = Freed::new();
	{
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		if limit > state.max_key {
			return;
		}
		let keys: Vec<u32> = state
			.pages
			.keys()
			.copied()
			.filter(|key| *key >= limit)
			.collect();
		// Pinned pages at or above the limit are implicitly unpinned and discarded too.
		for key in keys {
			take_page(pager, &mut shard, handle_addr, state, key, &mut freed);
		}
		state.max_key = limit.saturating_sub(1);
	}
	pager.release(freed);
}

unsafe extern "C" fn x_destroy(cache: *mut sqlite3_pcache) {
	// SAFETY: SQLite passes a live cache, here for the last time. The handle is freed only after
	// every LRU key naming it is removed under its shard's lock, so no eviction can reach it.
	let handle_ptr = cache as *mut CacheHandle;
	let handle = unsafe { &*handle_ptr };
	let pager = handle.cache;
	let handle_addr = handle_ptr as usize;
	let mut freed = Freed::new();
	{
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		let keys: Vec<u32> = state.pages.keys().copied().collect();
		for key in keys {
			take_page(pager, &mut shard, handle_addr, state, key, &mut freed);
		}
	}
	pager.release(freed);
	// SAFETY: created by `Box::into_raw` in `create`, and no lock or LRU key refers to it now.
	drop(unsafe { Box::from_raw(handle_ptr) });
}

unsafe extern "C" fn x_shrink(cache: *mut sqlite3_pcache) {
	// SAFETY: SQLite passes a live cache.
	let handle = unsafe { handle(cache) };
	let pager = handle.cache;
	let handle_addr = handle as *const CacheHandle as usize;
	let mut freed = Freed::new();
	{
		let mut shard = pager.shards[handle.shard].lock();
		// SAFETY: the state is reached under its shard's lock.
		let state = unsafe { &mut *handle.state.get() };
		if !state.purgeable {
			return;
		}
		let unpinned: Vec<u32> = state
			.pages
			.iter()
			// SAFETY: slots in the map are alive.
			.filter(|(_, slot)| !unsafe { slot.as_ref() }.pinned)
			.map(|(key, _)| *key)
			.collect();
		for key in unpinned {
			take_page(pager, &mut shard, handle_addr, state, key, &mut freed);
		}
	}
	pager.release(freed);
}


