//! Runs each database's SQLite commands on a shared pool of plain OS threads.
//!
//! Every database is a serial queue: at most one pool thread holds its connection at a time,
//! runs one command, and requeues the database if more work is waiting. A connection with an
//! open explicit transaction stays with its database between commands and holds no thread.
//! Pool threads must be plain OS threads, never Tokio workers, because VFS callbacks block on
//! the Tokio runtime with `block_on` and sleep with `std::thread::sleep`.

use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	error::Error,
	fmt,
	sync::{
		Arc, Weak,
		atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
	},
	thread::JoinHandle,
	time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use libsqlite3_sys::{
	SQLITE_OPEN_CREATE, SQLITE_OPEN_READWRITE, sqlite3_compileoption_used, sqlite3_get_autocommit,
};
use parking_lot::{Condvar, Mutex};
use tokio::sync::{Notify, oneshot, watch};

use crate::{
	guard::{ConnectionGuard, InstalledGuard},
	optimization_flags::sqlite_optimization_flags,
	page_buffer::PageBudget,
	query::{
		BindParam, ExecuteResult, QueryResult, exec_statements, exec_statements_profiled,
		execute_single_statement, execute_single_statement_profiled,
	},
	vfs::{
		NativeConnection, NativeVfsHandle, SqliteCommitIndeterminateError, SqliteOperationProfile,
		SqliteRoundTripCounts, SqliteVfsMetrics, configure_connection_for_database,
		open_connection, sqlite_exec,
	},
};

// Keep the first worker version intentionally fixed-size. A full queue maps to
// actor.overloaded so callers get explicit backpressure instead of hidden work.
pub const SQLITE_WORKER_QUEUE_CAPACITY: usize = 128;
const SQLITE_WORKER_CLOSE_TIMEOUT: Duration = Duration::from_secs(5);
// Transactions slower than this are logged at warn so operators can spot SQLite
// work that is stalling on engine round trips. Everything below still logs at
// debug and records its round trip count to the histogram.
const SQLITE_SLOW_TRANSACTION_THRESHOLD: Duration = Duration::from_millis(2500);

/// Environment variable that sizes the process pool.
pub const SQLITE_WORKER_THREADS_ENV: &str = "RIVETKIT_SQLITE_WORKER_THREADS";
pub const DEFAULT_SQLITE_WORKER_THREADS: usize = 256;
const MAX_SQLITE_WORKER_THREADS: usize = 4096;
/// Explicit, so thousands of databases never imply thousands of 2 MiB default stacks.
const SQLITE_WORKER_STACK_BYTES: usize = 512 * 1024;
/// Warn when work has waited behind a pool with no free thread for this long.
const SQLITE_POOL_SATURATED_WARN: Duration = Duration::from_secs(1);
/// Shortest time between two allocator trims. A trim walks every allocator arena under its
/// lock and returns freed pages to the OS, which interrupts every thread of the process, so a
/// burst of closing databases asks for many trims and gets one.
const ALLOCATOR_TRIM_INTERVAL: Duration = Duration::from_secs(5);
/// Background databases get one pool turn in this many while foreground work waits.
const BACKGROUND_TURN_EVERY: usize = 4;
/// Due idle releases one pool thread collects per wake.
const IDLE_RELEASES_PER_WAKE: usize = 64;

const STATE_RUNNING: u8 = 0;
const STATE_CLOSING: u8 = 1;
const STATE_CLOSED: u8 = 2;
const STATE_DEAD: u8 = 3;

/// Process-level pool gauges. Implemented by the runtime that owns the pool.
pub trait SqlitePoolMetrics: Send + Sync {
	/// The configured thread cap.
	fn set_pool_threads(&self, threads: u64);

	/// Threads currently running a database action.
	fn set_pool_busy(&self, busy: u64);

	/// Databases waiting for a thread.
	fn set_pool_runnable(&self, runnable: u64);

	/// Time a database waited between becoming runnable and a thread taking it.
	fn observe_pool_queue_wait(&self, wait_ns: u64);

	/// Time a caller waited to take one of the pool's locks: `run_queue` or `idle`.
	fn observe_pool_lock_wait(&self, lock: &'static str, wait_ns: u64);

	/// Configured capacity, in pages, of the process-wide VFS page budget.
	fn set_page_buffer_capacity_pages(&self, _pages: u64) {}

	/// Pages the process-wide VFS page budget holds across every database.
	fn set_page_buffer_pages(&self, _pages: u64) {}

	/// Configured byte budget of the process-wide SQLite pager cache.
	fn set_pager_cache_capacity_bytes(&self, _bytes: u64) {}

	/// Bytes the process-wide SQLite pager cache holds across every connection.
	fn set_pager_cache_bytes(&self, _bytes: u64) {}

	/// Pages the process-wide SQLite pager cache holds across every connection.
	fn set_pager_cache_pages(&self, _pages: u64) {}

	/// Bytes of in-memory and temporary database pages the process-wide SQLite pager cache holds
	/// outside its budget.
	fn set_pager_cache_unbudgeted_bytes(&self, _bytes: u64) {}
}

/// A bounded pool of plain OS threads shared by every database of one runtime process. The
/// pool spawns threads lazily, up to its cap, only when no thread is free. Cloning shares the
/// pool; it shuts down once the owner and every database handle have dropped their clones.
#[derive(Clone)]
pub struct SqlitePool {
	owner: Arc<PoolOwner>,
	page_budget: Arc<PageBudget>,
}

/// Signals shutdown when the last pool clone drops. Threads finish their current action and
/// exit; a detached joiner reaps them so dropping never blocks an async caller.
struct PoolOwner {
	shared: Arc<PoolShared>,
}

struct PoolShared {
	max_threads: usize,
	/// The run queue. Threads wait on `wake` with this lock held.
	state: Mutex<PoolState>,
	/// Idle release bookkeeping, taken only when a database first goes idle, when one is
	/// cancelled, and by a thread looking for due releases. Lock order: `state`, then `idle`.
	idle: Mutex<IdleQueue<DatabaseSlot>>,
	/// Origin of every slot's `last_used_ns`.
	epoch: Instant,
	wake: Condvar,
	metrics: Option<Arc<dyn SqlitePoolMetrics>>,
	
}

/// Databases waiting for their idle release, each queued at most once.
///
/// An entry is a check time, not the release time: a database used again after it was
/// queued keeps its entry, and the check moves it to its new due time instead of releasing.
struct IdleQueue<T> {
	queue: BTreeSet<(Instant, u64)>,
	entries: BTreeMap<u64, (Instant, Weak<T>)>,
}

impl<T> Default for IdleQueue<T> {
	fn default() -> Self {
		Self {
			queue: BTreeSet::new(),
			entries: BTreeMap::new(),
		}
	}
}

impl<T> IdleQueue<T> {
	/// Queues `id` at `at`, replacing any earlier entry. Returns whether it became the head,
	/// which is the only case in which a sleeping thread must wake.
	fn insert(&mut self, id: u64, at: Instant, item: Weak<T>) -> bool {
		if let Some((old, _)) = self.entries.insert(id, (at, item)) {
			self.queue.remove(&(old, id));
		}
		self.queue.insert((at, id));
		self.queue.first() == Some(&(at, id))
	}

	fn cancel(&mut self, id: u64) {
		if let Some((at, _)) = self.entries.remove(&id) {
			self.queue.remove(&(at, id));
		}
	}

	fn head(&self) -> Option<Instant> {
		self.queue.first().map(|(at, _)| *at)
	}

	

	/// Removes up to `limit` entries due at `now`. `due_at` reads a live item's current due
	/// time: one still in the future is queued again at that time, and the rest are returned.
	fn pop_due(
		&mut self,
		now: Instant,
		limit: usize,
		due_at: impl Fn(&Arc<T>) -> Option<Instant>,
	) -> Vec<Arc<T>> {
		let mut due = Vec::new();
		while due.len() < limit {
			let Some(&(at, id)) = self.queue.first() else {
				break;
			};
			if at > now {
				break;
			}
			self.queue.pop_first();
			let Some((_, item)) = self.entries.remove(&id) else {
				continue;
			};
			let Some(item) = item.upgrade() else {
				continue;
			};
			match due_at(&item) {
				Some(later) if later > now => {
					self.entries.insert(id, (later, Arc::downgrade(&item)));
					self.queue.insert((later, id));
				}
				Some(_) | None => due.push(item),
			}
		}
		due
	}
}

/// Databases waiting for a thread, in two classes. Foreground work (an actor starting or
/// serving) goes first, so a mass of actors going to sleep cannot hold a new actor's start
/// behind their teardown. Background work still gets one turn in every
/// `BACKGROUND_TURN_EVERY`, so a busy node keeps finishing its sleeps.
struct RunQueue<T> {
	foreground: VecDeque<(T, Instant)>,
	background: VecDeque<(T, Instant)>,
	/// Foreground turns taken in a row while background work waited.
	foreground_streak: usize,
}

impl<T> Default for RunQueue<T> {
	fn default() -> Self {
		Self {
			foreground: VecDeque::new(),
			background: VecDeque::new(),
			foreground_streak: 0,
		}
	}
}

impl<T> RunQueue<T> {
	fn push(&mut self, item: T, background: bool, now: Instant) {
		if background {
			self.background.push_back((item, now));
		} else {
			self.foreground.push_back((item, now));
		}
	}

	fn pop(&mut self) -> Option<(T, Instant)> {
		let background_turn = !self.background.is_empty()
			&& (self.foreground.is_empty() || self.foreground_streak >= BACKGROUND_TURN_EVERY);
		if background_turn {
			self.foreground_streak = 0;
			return self.background.pop_front();
		}
		let next = self.foreground.pop_front()?;
		if self.background.is_empty() {
			self.foreground_streak = 0;
		} else {
			self.foreground_streak += 1;
		}
		Some(next)
	}

	fn len(&self) -> usize {
		self.foreground.len() + self.background.len()
	}

	fn is_empty(&self) -> bool {
		self.foreground.is_empty() && self.background.is_empty()
	}
}

#[derive(Default)]
struct PoolState {
	runnable: RunQueue<Arc<DatabaseSlot>>,
	/// Connections whose handles were dropped without a close. They are closed on a pool
	/// thread because closing may flush through the VFS.
	orphans: Vec<NativeConnection>,
	threads: Vec<JoinHandle<()>>,
	/// Threads blocked waiting for work.
	waiting: usize,
	busy: usize,
	saturation: Saturation,
	next_database: u64,
	shutdown: bool,
	/// When the requested allocator trim may run, or `None` when none is requested.
	trim_due_at: Option<Instant>,
	/// When the last allocator trim finished.
	last_trim_at: Option<Instant>,
}

/// How long the pool has gone without a free thread while work waited. A thread that finishes
/// one action and takes the next queued one never becomes free, so the stretch continues
/// across task boundaries; it ends only when a thread waits for work.
#[derive(Default)]
struct Saturation {
	since: Option<Instant>,
	warned: bool,
}

impl Saturation {
	/// A thread is waiting for work, so the pool is not saturated.
	fn freed(&mut self) {
		self.since = None;
		self.warned = false;
	}

	/// Notes work waiting for a thread at `now`. `saturated` says every thread the pool may
	/// have exists and none is free. Returns true once per stretch, when it reaches
	/// `SQLITE_POOL_SATURATED_WARN`.
	fn work_waiting(&mut self, saturated: bool, now: Instant) -> bool {
		if !saturated {
			return false;
		}
		let since = *self.since.get_or_insert(now);
		if self.warned || now.duration_since(since) < SQLITE_POOL_SATURATED_WARN {
			return false;
		}
		self.warned = true;
		true
	}
}

/// One database's serial queue. The connection is either parked here or held by exactly one
/// pool thread.
struct DatabaseSlot {
	opening_trace: Option<WorkerTrace>,
	id: u64,
	inner: Arc<SqliteWorkerInner>,
	shared: Arc<PoolShared>,
	vfs: NativeVfsHandle,
	file_name: String,
	idle_release: Option<Duration>,
	/// When this database last finished a command, in nanoseconds since the pool epoch.
	last_used_ns: AtomicU64,
	/// Whether this database has an entry in the pool's idle queue.
	idle_queued: AtomicBool,
	/// Whether this database's work is background work, such as an actor going to sleep. Read
	/// each time the database is queued.
	background: AtomicBool,
	/// Installed on the connection when it opens; boxed so SQLite's pointer to it stays valid.
	guard: Option<Box<InstalledGuard>>,
	work: Mutex<SlotWork>,
}

/// Diagnostics travel with the existing bounded command queue. Neither SQL nor its
/// parameters are recorded; disabled TRACE retains no span or subscriber.
#[derive(Clone)]
struct WorkerTrace {
	span: tracing::Span,
	dispatch: tracing::Dispatch,
}
impl Drop for WorkerTrace {
	fn drop(&mut self) {
		// The registry closes parent references through the ambient subscriber.
		// Retained contexts can be released by another worker or request collector.
		tracing::dispatcher::with_default(&self.dispatch, || {
			drop(std::mem::replace(&mut self.span, tracing::Span::none()));
		});
	}
}

impl WorkerTrace {
	fn capture() -> Option<Self> {
		tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE).then(|| Self {
			span: tracing::Span::current(),
			dispatch: tracing::dispatcher::get_default(Clone::clone),
		})
	}
}
struct QueuedCommand {
	command: SqliteCommand,
	trace: Option<WorkerTrace>,
}

struct SlotWork {
	commands: VecDeque<QueuedCommand>,
	/// In the runnable queue or held by a thread.
	scheduled: bool,
	/// The idle release came due; the next action releases caches.
	idle_due: bool,
	connection: ConnectionSlot,
	close_requested_at: Option<Instant>,
}

enum ConnectionSlot {
	/// Not yet opened; the first action opens it and reports readiness.
	Unopened(Option<oneshot::Sender<Result<()>>>),
	/// Parked between actions, possibly inside an explicit transaction.
	Parked(OpenConnection),
	/// A pool thread holds it.
	Running,
	/// Closed, failed to open, or lost to a panic.
	Gone,
}

struct OpenConnection {
	db: NativeConnection,
	transaction: Option<TransactionTracker>,
	/// An uncork that arrived inside an explicit transaction. It runs once the
	/// connection returns to autocommit.
	pending_uncork: Option<oneshot::Sender<Result<()>>>,
}

/// What a pool thread does with a database it took.
enum Action {
	Open(Option<oneshot::Sender<Result<()>>>),
	Run(OpenConnection, QueuedCommand),
	Release(OpenConnection),
	Close(Option<OpenConnection>),
}

/// Work a pool thread takes from the pool.
enum PoolTask {
	Database(Arc<DatabaseSlot>, Instant),
	IdleReleases(Vec<Arc<DatabaseSlot>>),
	Orphan(NativeConnection),
	TrimAllocator,
}

/// The process-wide VFS page budget, sized from the optimization flags. Disabled page caching
/// gives it no capacity.
fn process_page_budget(metrics: Option<Arc<dyn SqlitePoolMetrics>>) -> Arc<PageBudget> {
	let flags = sqlite_optimization_flags();
	let caches_pages = flags.vfs_page_cache_mode.caches_any_pages();
	let (capacity, ttl_ms) = if caches_pages {
		(
			flags.vfs_page_buffer_process_pages,
			flags.vfs_staging_cache_ttl_ms,
		)
	} else {
		(0, 0)
	};
	PageBudget::new(capacity, ttl_ms, metrics)
}

impl SqlitePool {
	/// Reads the thread cap from `RIVETKIT_SQLITE_WORKER_THREADS`.
	pub fn from_env(metrics: Option<Arc<dyn SqlitePoolMetrics>>) -> Self {
		let threads = std::env::var(SQLITE_WORKER_THREADS_ENV)
			.ok()
			.and_then(|value| value.trim().parse::<usize>().ok())
			.filter(|threads| *threads > 0)
			.unwrap_or(DEFAULT_SQLITE_WORKER_THREADS)
			.min(MAX_SQLITE_WORKER_THREADS);
		Self::new(threads, metrics)
	}

	/// Starts no threads; the first database action spawns the first one.
	pub fn new(max_threads: usize, metrics: Option<Arc<dyn SqlitePoolMetrics>>) -> Self {
		let max_threads = max_threads.max(1);
		if let Some(metrics) = &metrics {
			metrics.set_pool_threads(max_threads as u64);
		}
		let page_budget = process_page_budget(metrics.clone());
		// Every connection opens on a pool, so the first pool installs the pager cache before
		// SQLite initializes.
		crate::pager_cache::install(
			sqlite_optimization_flags()
				.pager_process_cache_kib
				.saturating_mul(1024),
			metrics.clone(),
		);
		warn_if_direct_overflow_reads();
		Self {
			page_budget,
			owner: Arc::new(PoolOwner {
				shared: Arc::new(PoolShared {
					max_threads,
					state: Mutex::new(PoolState::default()),
					idle: Mutex::new(IdleQueue::default()),
					epoch: Instant::now(),
					wake: Condvar::new(),
					metrics,
					
				}),
			}),
		}
	}

	/// The page budget every database on this pool shares.
	pub fn page_budget(&self) -> &Arc<PageBudget> {
		&self.page_budget
	}

	/// Threads spawned so far, never more than the cap.
	/// Returns freed native memory to the OS soon, on a pool thread, coalescing requests so
	/// trims run at most once per `ALLOCATOR_TRIM_INTERVAL`. Call after releasing a database.
	pub fn request_allocator_trim(&self) {
		self.owner.shared.request_allocator_trim();
	}

	pub fn live_threads(&self) -> usize {
		self.owner.shared.state.lock().threads.len()
	}

	pub fn max_threads(&self) -> usize {
		self.owner.shared.max_threads
	}

	

	
}

impl Drop for PoolOwner {
	fn drop(&mut self) {
		let threads = {
			let mut state = self.shared.state.lock();
			state.shutdown = true;
			std::mem::take(&mut state.threads)
		};
		self.shared.wake.notify_all();
		if threads.is_empty() {
			return;
		}
		// Joining here could block an async caller behind a thread inside a VFS callback.
		let joiner = std::thread::Builder::new()
			.name("sqlite-pool-join".to_owned())
			.stack_size(64 * 1024)
			.spawn(move || {
				for thread in threads {
					if thread.join().is_err() {
						tracing::error!("sqlite pool thread panicked during shutdown");
					}
				}
			});
		if let Err(error) = joiner {
			tracing::error!(
				?error,
				"failed to spawn sqlite pool joiner; threads exit unjoined"
			);
		}
	}
}

impl PoolShared {
	/// Queues a database for a thread, spawning one if none is free and the cap allows.
	fn schedule(self: &Arc<Self>, slot: Arc<DatabaseSlot>) {
		let mut state = self.lock_state();
		let now = Instant::now();
		let background = slot.background.load(Ordering::Acquire);
		state.runnable.push(slot, background, now);
		if let Some(metrics) = &self.metrics {
			metrics.set_pool_runnable(state.runnable.len() as u64);
		}
		let mut failed_slots = Vec::new();
		if state.waiting == 0 && state.threads.len() < self.max_threads && !state.shutdown {
			if !self.spawn_thread(&mut state) && state.threads.is_empty() {
				// No worker can take these slots, including their pending ready senders.
				while let Some((slot, _)) = state.runnable.pop() {
					failed_slots.push(slot);
				}
				if let Some(metrics) = &self.metrics {
					metrics.set_pool_runnable(0);
				}
			}
		}
		self.note_waiting_work(&mut state, now);
		drop(state);
		for slot in failed_slots {
			slot.fail_without_worker();
		}
		self.wake.notify_one();
	}

	/// Warns once per stretch in which queued work has waited behind a pool whose every
	/// thread is busy for longer than `SQLITE_POOL_SATURATED_WARN`.
	fn note_waiting_work(&self, state: &mut PoolState, now: Instant) {
		let saturated = state.waiting == 0 && state.threads.len() >= self.max_threads;
		if state.saturation.work_waiting(saturated, now) {
			tracing::warn!(
				threads = self.max_threads,
				runnable = state.runnable.len(),
				"every sqlite pool thread has been busy for over a second while work waited; raise RIVETKIT_SQLITE_WORKER_THREADS or look for slow VFS round trips"
			);
		}
	}

	fn spawn_thread(self: &Arc<Self>, state: &mut PoolState) -> bool {
		let shared = Arc::clone(self);
		let index = state.threads.len();
		
		let spawned = std::thread::Builder::new()
			.name(format!("sqlite-pool-{index}"))
			.stack_size(SQLITE_WORKER_STACK_BYTES)
			.spawn(move || pool_thread(shared));
		match spawned {
			Ok(thread) => {
				state.threads.push(thread);
				true
			}
			Err(error) => {
				// Existing threads still drain the queue; a pool with no thread cannot.
				tracing::error!(
					?error,
					threads = state.threads.len(),
					"failed to spawn sqlite pool thread"
				);
				false
			}
		}
	}

	/// Takes the run queue lock, recording how long it waited.
	fn lock_state(&self) -> parking_lot::MutexGuard<'_, PoolState> {
		self.timed_lock(&self.state, "run_queue")
	}

	fn lock_idle(&self) -> parking_lot::MutexGuard<'_, IdleQueue<DatabaseSlot>> {
		self.timed_lock(&self.idle, "idle")
	}

	fn timed_lock<'a, T>(
		&self,
		mutex: &'a Mutex<T>,
		lock: &'static str,
	) -> parking_lot::MutexGuard<'a, T> {
		if let Some(guard) = mutex.try_lock() {
			return guard;
		}
		let started = Instant::now();
		let guard = mutex.lock();
		if let Some(metrics) = &self.metrics {
			metrics.observe_pool_lock_wait(lock, started.elapsed().as_nanos() as u64);
		}
		guard
	}

	fn now_ns(&self) -> u64 {
		self.epoch.elapsed().as_nanos().min(u64::MAX as u128) as u64
	}

	/// Records a finished command and queues the database's idle check once, when it first
	/// goes idle. Later commands only move `last_used_ns`; the check reads it lazily.
	fn note_used(&self, slot: &Arc<DatabaseSlot>, idle: Duration) {
		slot.last_used_ns.store(self.now_ns(), Ordering::SeqCst);
		if slot.idle_queued.swap(true, Ordering::SeqCst) {
			return;
		}
		let at = Instant::now() + idle;
		let head = self.lock_idle().insert(slot.id, at, Arc::downgrade(slot));
		if head {
			// A thread waiting for an idle check holds the run queue lock from reading the
			// head until it waits, so taking that lock first means the notify cannot be lost.
			drop(self.lock_state());
			self.wake.notify_one();
		}
	}

	/// The time `slot`'s idle release is due, from its last use.
	fn idle_due_at(&self, slot: &DatabaseSlot) -> Option<Instant> {
		let idle = slot.idle_release?;
		let last_used = Duration::from_nanos(slot.last_used_ns.load(Ordering::SeqCst));
		Some(self.epoch + last_used + idle)
	}

	fn cancel_idle(&self, id: u64) {
		self.lock_idle().cancel(id);
	}

	/// Takes the idle checks due at `now`. A database used since it was queued is queued
	/// again at its new due time; the others are returned for release.
	fn due_idle_releases(&self, now: Instant) -> Vec<Arc<DatabaseSlot>> {
		let mut idle = self.lock_idle();
		let due = idle.pop_due(now, IDLE_RELEASES_PER_WAKE, |slot| self.idle_due_at(slot));
		let mut released = Vec::with_capacity(due.len());
		for slot in due {
			slot.idle_queued.store(false, Ordering::SeqCst);
			// A command that finished after the check read its last use saw the database
			// still queued; queue it again from that use instead of releasing it.
			match self.idle_due_at(&slot) {
				Some(at) if at > now => {
					if !slot.idle_queued.swap(true, Ordering::SeqCst) {
						idle.insert(slot.id, at, Arc::downgrade(&slot));
					}
				}
				Some(_) | None => released.push(slot),
			}
		}
		released
	}

	fn orphan(&self, connection: NativeConnection) {
		let mut state = self.lock_state();
		state.orphans.push(connection);
		drop(state);
		self.wake.notify_one();
	}

	/// Waits for the next task, or returns `None` at shutdown once the queue is empty.
	fn next_task(&self) -> Option<PoolTask> {
		let mut state = self.lock_state();
		loop {
			if let Some(connection) = state.orphans.pop() {
				return Some(PoolTask::Orphan(connection));
			}
			if let Some((slot, queued_at)) = state.runnable.pop() {
				if let Some(metrics) = &self.metrics {
					metrics.set_pool_runnable(state.runnable.len() as u64);
				}
				if !state.runnable.is_empty() {
					self.note_waiting_work(&mut state, Instant::now());
				}
				return Some(PoolTask::Database(slot, queued_at));
			}
			let due = self.due_idle_releases(Instant::now());
			if !due.is_empty() {
				return Some(PoolTask::IdleReleases(due));
			}
			if state.trim_due_at.is_some_and(|at| at <= Instant::now()) {
				state.trim_due_at = None;
				return Some(PoolTask::TrimAllocator);
			}
			if state.shutdown {
				return None;
			}
			state.waiting += 1;
			state.saturation.freed();
			let idle_head = self.lock_idle().head();
			let head = match (idle_head, state.trim_due_at) {
				(Some(idle), Some(trim)) => Some(idle.min(trim)),
				(idle, trim) => idle.or(trim),
			};
			match head {
				Some(at) => {
					self.wake.wait_until(&mut state, at);
				}
				None => self.wake.wait(&mut state),
			}
			state.waiting -= 1;
		}
	}

	/// Asks a pool thread to return freed native memory to the OS, at most once per
	/// `ALLOCATOR_TRIM_INTERVAL`. A request while one is pending joins it.
	fn request_allocator_trim(self: &Arc<Self>) {
		let mut state = self.lock_state();
		if state.trim_due_at.is_some() || state.shutdown {
			return;
		}
		let now = Instant::now();
		let due = state
			.last_trim_at
			.map_or(now, |last| (last + ALLOCATOR_TRIM_INTERVAL).max(now));
		state.trim_due_at = Some(due);
		if state.waiting == 0 && state.threads.is_empty() {
			self.spawn_thread(&mut state);
		}
		drop(state);
		// A waiting thread recomputes its wake time from the new due time.
		self.wake.notify_one();
	}

	/// Publishes the number of threads running an action.
	fn set_busy(&self, delta: isize) {
		let mut state = self.lock_state();
		state.busy = state.busy.saturating_add_signed(delta);
		if let Some(metrics) = &self.metrics {
			metrics.set_pool_busy(state.busy as u64);
		}
	}
}

/// Returns the allocator's freed pages to the OS. glibc keeps freed memory in its arenas, so a
/// node that closed many databases would otherwise hold their peak memory.
#[cfg(all(unix, target_env = "gnu"))]
fn trim_allocator() {
	unsafe extern "C" {
		fn malloc_trim(pad: usize) -> i32;
	}

	let started = Instant::now();
	let rc = unsafe { malloc_trim(0) };
	tracing::debug!(
		rc,
		duration_us = started.elapsed().as_micros() as u64,
		"trimmed native allocator"
	);
}

#[cfg(not(all(unix, target_env = "gnu")))]
fn trim_allocator() {}

fn pool_thread(shared: Arc<PoolShared>) {
	while let Some(task) = shared.next_task() {
		shared.set_busy(1);
		match task {
			PoolTask::Database(slot, queued_at) => {
				if let Some(metrics) = &shared.metrics {
					metrics.observe_pool_queue_wait(queued_at.elapsed().as_nanos() as u64);
				}
				slot.run_one();
			}
			PoolTask::IdleReleases(slots) => {
				for slot in slots {
					slot.mark_idle_due();
				}
			}
			PoolTask::Orphan(connection) => drop(connection),
			PoolTask::TrimAllocator => {
				trim_allocator();
				shared.lock_state().last_trim_at = Some(Instant::now());
			}
		}
		shared.set_busy(-1);
	}
}

impl DatabaseSlot {
	/// Completes an open that no pool thread can service after thread creation failed.
	fn fail_without_worker(&self) {
		self.inner.state.store(STATE_DEAD, Ordering::Release);
		self.inner.explicit_transaction.send_replace(false);
		let mut work = self.work.lock();
		debug_assert!(matches!(work.connection, ConnectionSlot::Unopened(_)));
		work.connection = ConnectionSlot::Gone;
		work.scheduled = false;
		fail_queued_commands(&mut work.commands);
		drop(work);
		self.inner.closed.notify_waiters();
	}

	/// Queues one command, or fails at once when the database is closing or its queue is full.
	fn enqueue(self: &Arc<Self>, command: SqliteCommand) -> Result<()> {
		match self.inner.state.load(Ordering::Acquire) {
			STATE_RUNNING => {}
			STATE_CLOSING | STATE_CLOSED => return Err(sqlite_closing_error()),
			STATE_DEAD => return Err(sqlite_worker_dead_error()),
			other => return Err(anyhow!("unknown sqlite worker state {other}")),
		}
		let mut work = self.work.lock();
		if work.commands.len() >= SQLITE_WORKER_QUEUE_CAPACITY {
			drop(work);
			if let Some(metrics) = &self.inner.metrics {
				metrics.record_worker_queue_overload();
			}
			// SQL backpressure is actor backpressure. Waiting for capacity here
			// would let a single actor build hidden native SQLite backlog.
			return Err(SqliteWorkerOverloadedError.into());
		}
		work.commands.push_back(QueuedCommand {
			command,
			trace: WorkerTrace::capture(),
		});
		self.inner.record_queue_depth(work.commands.len() as u64);
		self.schedule_locked(work);
		Ok(())
	}

	/// Requests a close that runs after any command a thread is running now.
	fn request_close(self: &Arc<Self>) {
		let mut work = self.work.lock();
		work.close_requested_at.get_or_insert_with(Instant::now);
		self.schedule_locked(work);
	}

	fn mark_idle_due(self: &Arc<Self>) {
		let mut work = self.work.lock();
		if !matches!(work.connection, ConnectionSlot::Parked(_)) {
			return;
		}
		work.idle_due = true;
		self.schedule_locked(work);
	}

	/// Queues the database unless it is already queued or running, then releases the lock
	/// before taking the pool lock.
	fn schedule_locked(self: &Arc<Self>, mut work: parking_lot::MutexGuard<'_, SlotWork>) {
		if work.scheduled {
			return;
		}
		work.scheduled = true;
		drop(work);
		self.shared.schedule(Arc::clone(self));
	}

	/// Takes the connection and the next action, runs it without any lock, then parks the
	/// connection and requeues the database if more work is waiting.
	fn run_one(self: &Arc<Self>) {
		let action = {
			let mut work = self.work.lock();
			let closing = self.inner.state.load(Ordering::Acquire) == STATE_CLOSING;
			let connection = std::mem::replace(&mut work.connection, ConnectionSlot::Running);
			match connection {
				ConnectionSlot::Unopened(ready) if closing => {
					drop(ready);
					Some(Action::Close(None))
				}
				ConnectionSlot::Unopened(ready) => Some(Action::Open(ready)),
				ConnectionSlot::Parked(open) if closing => Some(Action::Close(Some(open))),
				ConnectionSlot::Parked(open) => match work.commands.pop_front() {
					Some(command) => {
						self.inner.record_queue_depth(work.commands.len() as u64);
						Some(Action::Run(open, command))
					}
					None if work.idle_due => Some(Action::Release(open)),
					None => {
						work.connection = ConnectionSlot::Parked(open);
						None
					}
				},
				ConnectionSlot::Running => {
					// Only the thread holding the database replaces `Running`.
					tracing::error!("sqlite database scheduled while a thread holds it");
					None
				}
				ConnectionSlot::Gone => {
					work.connection = ConnectionSlot::Gone;
					fail_queued_commands(&mut work.commands);
					None
				}
			}
		};
		let Some(action) = action else {
			self.finish_idle();
			return;
		};

		let ran_command = matches!(action, Action::Run(..));
		let released = matches!(action, Action::Release(_));
		let outcome =
			std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.perform(action)));
		let next = match outcome {
			Ok(next) => next,
			Err(panic) => {
				self.inner.state.store(STATE_DEAD, Ordering::Release);
				self.inner.explicit_transaction.send_replace(false);
				if let Some(metrics) = &self.inner.metrics {
					metrics.record_worker_crash();
					metrics.set_vfs_cached_bytes(0);
				}
				self.inner.closed.notify_waiters();
				tracing::error!(msg = panic_message(&panic), "sqlite worker panicked");
				ConnectionSlot::Gone
			}
		};
		let gone = matches!(next, ConnectionSlot::Gone);

		let requeue = {
			let mut work = self.work.lock();
			work.connection = next;
			if released {
				work.idle_due = false;
			}
			if gone {
				fail_queued_commands(&mut work.commands);
			}
			let more = !gone
				&& (!work.commands.is_empty()
					|| work.idle_due
					|| self.inner.state.load(Ordering::Acquire) == STATE_CLOSING);
			if !more {
				work.scheduled = false;
			}
			more
		};
		if gone {
			self.shared.cancel_idle(self.id);
		} else if ran_command && let Some(idle) = self.idle_release {
			self.shared.note_used(self, idle);
		}
		if requeue {
			self.shared.schedule(Arc::clone(self));
		}
	}

	/// Clears the scheduled flag when a thread found nothing to do.
	fn finish_idle(self: &Arc<Self>) {
		let requeue = {
			let mut work = self.work.lock();
			let more = !work.commands.is_empty()
				|| (matches!(work.connection, ConnectionSlot::Parked(_))
					&& self.inner.state.load(Ordering::Acquire) == STATE_CLOSING);
			if !more {
				work.scheduled = false;
			}
			more
		};
		if requeue {
			self.shared.schedule(Arc::clone(self));
		}
	}

	/// Runs one action on this thread and returns what to park in the slot.
	fn perform(&self, action: Action) -> ConnectionSlot {
		let trace = match &action {
			Action::Run(_, command) => command.trace.clone(),
			_ => self.opening_trace.clone(),
		};
		// This method is synchronous, including the blocking VFS callbacks. No span
		// guard crosses an async suspension or follows the connection to another thread.
		let _dispatch = trace
			.as_ref()
			.map(|trace| tracing::dispatcher::set_default(&trace.dispatch));
		let _span = trace.as_ref().map(|trace| trace.span.enter());
		let metrics = self.inner.metrics.as_deref();
		match action {
			Action::Open(ready) => match open_database_connection(&self.vfs, &self.file_name)
				.and_then(|db| {
					if let Some(guard) = &self.guard {
						guard.install(db.as_ptr())?;
					}
					Ok(db)
				}) {
				Ok(db) => {
					if let Some(ready) = ready {
						let _ = ready.send(Ok(()));
					}
					ConnectionSlot::Parked(OpenConnection {
						db,
						transaction: None,
						pending_uncork: None,
					})
				}
				Err(error) => {
					if let Some(ready) = ready {
						let _ = ready.send(Err(error));
					}
					self.inner.state.store(STATE_DEAD, Ordering::Release);
					self.inner.closed.notify_waiters();
					ConnectionSlot::Gone
				}
			},
			Action::Run(mut open, command) => {
				if let Some(guard) = &self.guard {
					guard.reset();
				}
				let _inflight = WorkerInflightGuard::new(metrics);
				let max_result_bytes = self.guard.as_ref().map(|guard| guard.max_result_bytes());
				run_command(
					&mut open.db,
					&self.vfs,
					command.command,
					max_result_bytes,
					metrics,
					&self.file_name,
					&mut open.transaction,
					&mut open.pending_uncork,
					&self.inner.explicit_transaction,
				);
				report_vfs_memory(&open.db, metrics);
				ConnectionSlot::Parked(open)
			}
			Action::Release(mut open) => {
				release_idle_memory(&mut open.db, metrics);
				ConnectionSlot::Parked(open)
			}
			Action::Close(open) => {
				let requested_at = {
					let mut work = self.work.lock();
					fail_queued_commands(&mut work.commands);
					work.close_requested_at
				};
				drop(open);
				if let Some(metrics) = metrics {
					metrics.set_vfs_cached_bytes(0);
					if let Some(requested_at) = requested_at {
						metrics
							.observe_worker_close_duration(requested_at.elapsed().as_nanos() as u64);
					}
				}
				self.inner.explicit_transaction.send_replace(false);
				self.inner.state.store(STATE_CLOSED, Ordering::Release);
				self.inner.closed.notify_waiters();
				ConnectionSlot::Gone
			}
		}
	}
}

impl Drop for DatabaseSlot {
	fn drop(&mut self) {
		let work = self.work.get_mut();
		fail_queued_commands(&mut work.commands);
		if let ConnectionSlot::Parked(open) =
			std::mem::replace(&mut work.connection, ConnectionSlot::Gone)
		{
			self.shared.orphan(open.db);
		}
		self.shared.cancel_idle(self.id);
		if self.inner.state.load(Ordering::Acquire) == STATE_RUNNING {
			if let Some(metrics) = &self.inner.metrics {
				metrics.record_worker_unclean_close();
				metrics.set_vfs_cached_bytes(0);
			}
			// The owning runtime lost the handle without running SQLite shutdown.
			tracing::error!("sqlite worker handle dropped without clean close");
			self.inner.explicit_transaction.send_replace(false);
			self.inner.state.store(STATE_CLOSED, Ordering::Release);
			self.inner.closed.notify_waiters();
		}
	}
}

fn fail_queued_commands(commands: &mut VecDeque<QueuedCommand>) {
	for command in commands.drain(..) {
		fail_command(command.command);
	}
}

#[derive(Clone)]
pub struct SqliteWorkerHandle {
	slot: Arc<DatabaseSlot>,
	_pool: SqlitePool,
}

struct SqliteWorkerInner {
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	state: AtomicU8,
	closed: Notify,
	ready: Mutex<Option<oneshot::Receiver<Result<()>>>>,
	/// Whether the connection is outside autocommit mode, published before each
	/// command's reply so a caller that sees the reply also sees the new state.
	/// Reset to `false` when the database stops because the connection is gone.
	explicit_transaction: watch::Sender<bool>,
}

enum SqliteCommand {
	Execute {
		sql: String,
		params: Option<Vec<BindParam>>,
		enqueued_at: Option<Instant>,
		reply: oneshot::Sender<Result<SqliteWorkerResult<ExecuteResult>>>,
	},
	Exec {
		sql: String,
		enqueued_at: Option<Instant>,
		reply: oneshot::Sender<Result<SqliteWorkerResult<QueryResult>>>,
	},
	/// Runs statements in order on one pool thread hold, stopping at the first that fails.
	ExecuteBatch {
		statements: Vec<(String, Option<Vec<BindParam>>)>,
		reply: oneshot::Sender<Result<SqliteWorkerResult<Vec<ExecuteResult>>>>,
	},
	/// Holds later commits in memory; see `NativeDatabase::cork_commits`.
	Cork { reply: oneshot::Sender<Result<()>> },
	/// Sends the corked commits as one; see `NativeDatabase::uncork_commits`.
	Uncork { reply: oneshot::Sender<Result<()>> },
	
	
}

#[derive(Debug)]
pub struct SqliteWorkerResult<T> {
	pub result: Result<T>,
	pub profile: SqliteOperationProfile,
}

struct WorkerInflightGuard<'a>(Option<&'a dyn SqliteVfsMetrics>);

impl<'a> WorkerInflightGuard<'a> {
	fn new(metrics: Option<&'a dyn SqliteVfsMetrics>) -> Self {
		if let Some(metrics) = metrics {
			metrics.set_worker_inflight(true);
		}
		Self(metrics)
	}
}

impl Drop for WorkerInflightGuard<'_> {
	fn drop(&mut self) {
		if let Some(metrics) = self.0 {
			metrics.set_worker_inflight(false);
		}
	}
}

/// Tracks an in-progress SQLite transaction so its wall-clock duration and the
/// network round trips it issued can be reported once it commits or rolls back.
///
/// A transaction begins on the first command run while none is open and ends
/// when the connection returns to autocommit mode (an autocommit statement
/// finishes, or an explicit `COMMIT`/`ROLLBACK` runs).
struct TransactionTracker {
	started_at: Instant,
	start_round_trips: SqliteRoundTripCounts,
	statement_count: u64,
}

/// Releases an idle database's pager and VFS page caches, unless an explicit transaction is
/// open: its pages are the transaction's working set and its next command needs them.
fn release_idle_memory(db: &mut NativeConnection, metrics: Option<&dyn SqliteVfsMetrics>) {
	if command_in_tx(db) {
		return;
	}
	if db.release_idle_memory() {
		report_vfs_memory(db, metrics);
	}
}

/// Reports the memory this database's VFS holds outside SQLite: its page caches and its read
/// page buffers.
fn report_vfs_memory(db: &NativeConnection, metrics: Option<&dyn SqliteVfsMetrics>) {
	if let Some(metrics) = metrics {
		metrics.set_vfs_cached_bytes(db.cached_page_bytes());
	}
}

impl SqliteWorkerHandle {
	/// Runs the database on a dedicated single-thread pool, for callers without a shared one.
	pub fn start(
		vfs: NativeVfsHandle,
		file_name: String,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Result<Self> {
		Self::start_on(&SqlitePool::new(1, None), vfs, file_name, metrics)
	}

	/// Registers the database with `pool`. A pool thread opens the connection; `wait_ready`
	/// reports the outcome.
	pub fn start_on(
		pool: &SqlitePool,
		vfs: NativeVfsHandle,
		file_name: String,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Result<Self> {
		Self::start_on_with_guard(pool, vfs, file_name, metrics, None)
	}

	/// Like [`SqliteWorkerHandle::start_on`], with `guard` installed on the connection and its
	/// work budget reset before every command.
	pub fn start_on_with_guard(
		pool: &SqlitePool,
		vfs: NativeVfsHandle,
		file_name: String,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
		guard: Option<ConnectionGuard>,
	) -> Result<Self> {
		let (ready_tx, ready_rx) = oneshot::channel();
		let inner = Arc::new(SqliteWorkerInner {
			metrics,
			state: AtomicU8::new(STATE_RUNNING),
			closed: Notify::new(),
			ready: Mutex::new(Some(ready_rx)),
			explicit_transaction: watch::Sender::new(false),
		});
		let shared = &pool.owner.shared;
		let id = {
			let mut state = shared.state.lock();
			if state.shutdown {
				return Err(anyhow!("sqlite pool is shutting down"));
			}
			state.next_database += 1;
			state.next_database
		};
		let idle_release = vfs.idle_release();
		let slot = Arc::new(DatabaseSlot {
			opening_trace: WorkerTrace::capture(),
			id,
			inner,
			shared: Arc::clone(shared),
			vfs,
			file_name,
			idle_release,
			last_used_ns: AtomicU64::new(0),
			idle_queued: AtomicBool::new(false),
			background: AtomicBool::new(false),
			guard: guard.map(InstalledGuard::new),
			work: Mutex::new(SlotWork {
				commands: VecDeque::new(),
				scheduled: false,
				idle_due: false,
				connection: ConnectionSlot::Unopened(Some(ready_tx)),
				close_requested_at: None,
			}),
		});
		let work = slot.work.lock();
		slot.schedule_locked(work);
		Ok(Self {
			slot,
			_pool: pool.clone(),
		})
	}

	fn inner(&self) -> &SqliteWorkerInner {
		&self.slot.inner
	}

	pub async fn wait_ready(&self) -> Result<()> {
		let ready = self.inner().ready.lock().take();
		let Some(ready) = ready else {
			return Ok(());
		};
		ready
			.await
			.map_err(|_| sqlite_worker_dead_error())?
			.map_err(|err| anyhow!("failed to initialize sqlite worker: {err}"))
	}

	/// Returns whether the connection has an open explicit transaction, such as
	/// one started by a manual `BEGIN`, as of the last completed command.
	/// Marks this database's later work as background work, which the pool runs after
	/// foreground work. Set when its actor starts shutting down.
	pub fn set_background(&self, background: bool) {
		self.slot.background.store(background, Ordering::Release);
	}

	pub fn explicit_transaction_open(&self) -> bool {
		*self.inner().explicit_transaction.borrow()
	}

	/// Subscribes to explicit transaction state changes. The value changes
	/// before the reply of the command that changed it is delivered.
	pub fn subscribe_explicit_transaction(&self) -> watch::Receiver<bool> {
		self.inner().explicit_transaction.subscribe()
	}

	pub async fn exec(&self, sql: String) -> Result<QueryResult> {
		self.exec_inner(sql, None).await?.result
	}

	pub async fn exec_profiled(&self, sql: String) -> Result<SqliteWorkerResult<QueryResult>> {
		self.exec_inner(sql, Some(Instant::now())).await
	}

	async fn exec_inner(
		&self,
		sql: String,
		enqueued_at: Option<Instant>,
	) -> Result<SqliteWorkerResult<QueryResult>> {
		let (reply, result) = oneshot::channel();
		self.slot.enqueue(SqliteCommand::Exec {
			sql,
			enqueued_at,
			reply,
		})?;
		result.await.map_err(|_| sqlite_worker_dead_error())?
	}

	pub async fn execute(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.execute_inner(sql, params, None).await?.result
	}

	pub async fn execute_profiled(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<SqliteWorkerResult<ExecuteResult>> {
		self.execute_inner(sql, params, Some(Instant::now())).await
	}

	async fn execute_inner(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
		enqueued_at: Option<Instant>,
	) -> Result<SqliteWorkerResult<ExecuteResult>> {
		let (reply, result) = oneshot::channel();
		self.slot.enqueue(SqliteCommand::Execute {
			sql,
			params,
			enqueued_at,
			reply,
		})?;
		result.await.map_err(|_| sqlite_worker_dead_error())?
	}

	/// Runs `statements` in order as one command, so they take one turn on the pool instead of
	/// one turn each. Statements after the first failure do not run. The batch does not open a
	/// transaction of its own: run it inside an explicit transaction for atomicity.
	pub async fn execute_batch(
		&self,
		statements: Vec<(String, Option<Vec<BindParam>>)>,
	) -> Result<Vec<ExecuteResult>> {
		let (reply, result) = oneshot::channel();
		self.slot
			.enqueue(SqliteCommand::ExecuteBatch { statements, reply })?;
		result
			.await
			.map_err(|_| sqlite_worker_dead_error())??
			.result
	}

	/// Holds every later commit in memory until `uncork_commits`, which sends them to depot as
	/// one commit. Commands queue in order, so the transactions after this call are the ones
	/// held. None of them is durable until uncorking succeeds.
	pub async fn cork_commits(&self) -> Result<()> {
		let (reply, result) = oneshot::channel();
		self.slot.enqueue(SqliteCommand::Cork { reply })?;
		result.await.map_err(|_| sqlite_worker_dead_error())?
	}

	/// Sends the transactions committed since `cork_commits` to depot as one commit. Inside an
	/// explicit transaction it waits until the transaction ends, so the commit includes it.
	pub async fn uncork_commits(&self) -> Result<()> {
		let (reply, result) = oneshot::channel();
		self.slot.enqueue(SqliteCommand::Uncork { reply })?;
		result.await.map_err(|_| sqlite_worker_dead_error())?
	}

	pub async fn close(&self) -> Result<()> {
		self.close_with_timeout(Some(SQLITE_WORKER_CLOSE_TIMEOUT))
			.await
	}

	/// Closes the database and waits, without a timeout, until no pool thread holds it.
	pub async fn close_and_wait(&self) -> Result<()> {
		self.close_with_timeout(None).await
	}

	async fn close_with_timeout(&self, close_timeout: Option<Duration>) -> Result<()> {
		let start = Instant::now();
		// Close is a control path, not SQL work, so it bypasses the bounded command queue.
		// Queued commands fail; a command a thread is running finishes first.
		if self.inner().mark_closing() {
			self.slot.request_close();
		}

		let wait_closed = async {
			loop {
				let closed = self.inner().closed.notified();
				match self.inner().state.load(Ordering::Acquire) {
					STATE_CLOSED => return Ok(()),
					STATE_DEAD => return Err(sqlite_worker_dead_error()),
					STATE_RUNNING | STATE_CLOSING => closed.await,
					other => return Err(anyhow!("unknown sqlite worker state {other}")),
				}
			}
		};

		match close_timeout {
			Some(close_timeout) => match tokio::time::timeout(close_timeout, wait_closed).await {
				Ok(result) => result,
				Err(_) => {
					if let Some(metrics) = &self.inner().metrics {
						metrics.record_worker_close_timeout();
					}
					// The pool still owns the connection and closes it after the running
					// command; the VFS must not be dropped before then.
					Err(SqliteWorkerCloseTimeoutError.into())
				}
			},
			None => {
				tokio::pin!(wait_closed);
				tokio::select! {
					result = &mut wait_closed => result,
					_ = tokio::time::sleep(SQLITE_WORKER_CLOSE_TIMEOUT) => {
						if let Some(metrics) = &self.inner().metrics {
							metrics.record_worker_close_timeout();
						}
						tracing::warn!(
							duration_ms = start.elapsed().as_millis() as u64,
							"waiting for SQLite worker to finish strict close",
						);
						wait_closed.await
					}
				}
			}
		}
	}

	pub async fn wait_for_failure(&self) -> bool {
		loop {
			let closed = self.inner().closed.notified();
			match self.inner().state.load(Ordering::Acquire) {
				STATE_DEAD => return true,
				STATE_CLOSED => return false,
				STATE_RUNNING | STATE_CLOSING => closed.await,
				_ => return true,
			}
		}
	}

	

	

	
}

impl SqliteWorkerInner {
	fn mark_closing(&self) -> bool {
		self.state
			.compare_exchange(
				STATE_RUNNING,
				STATE_CLOSING,
				Ordering::AcqRel,
				Ordering::Acquire,
			)
			.is_ok()
	}

	fn record_queue_depth(&self, depth: u64) {
		if let Some(metrics) = &self.metrics {
			metrics.set_worker_queue_depth(depth);
		}
	}
}

/// Warns when SQLite was built with direct overflow reads, which `LIBSQLITE3_FLAGS` in
/// `.cargo/config.toml` turns off. A build that misses that config reads whole overflow pages
/// straight from the file, past SQLite's page cache, so the VFS hands each one over and drops it
/// while nothing caches it, and every read of a large row fetches its pages from depot again.
fn warn_if_direct_overflow_reads() {
	// SAFETY: the argument is a NUL-terminated static string, and the call only scans SQLite's
	// static list of compile options.
	let enabled = unsafe { sqlite3_compileoption_used(c"DIRECT_OVERFLOW_READ".as_ptr()) } != 0;
	if enabled {
		tracing::warn!(
			"sqlite was built with SQLITE_DIRECT_OVERFLOW_READ, likely without the repo's cargo config, so reads of large rows bypass the page cache and refetch from depot"
		);
	}
}

fn open_database_connection(vfs: &NativeVfsHandle, file_name: &str) -> Result<NativeConnection> {
	let connection = open_connection(
		vfs.clone(),
		file_name,
		SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
	)
	.map_err(anyhow::Error::msg)?;
	configure_connection_for_database(connection.as_ptr(), vfs, file_name)?;
	Ok(connection)
}

#[allow(clippy::too_many_arguments)]
fn run_command(
	db: &mut NativeConnection,
	vfs: &NativeVfsHandle,
	command: SqliteCommand,
	max_result_bytes: Option<u64>,
	metrics: Option<&dyn SqliteVfsMetrics>,
	file_name: &str,
	transaction: &mut Option<TransactionTracker>,
	pending_uncork: &mut Option<oneshot::Sender<Result<()>>>,
	explicit_transaction: &watch::Sender<bool>,
) {
	// A storage failure belongs to the command whose statement saw it. One that a command
	// left unreported, such as an uncork's, must not relabel a later command's own failure.
	let _ = vfs.take_storage_error();
	run_command_inner(
		db,
		vfs,
		command,
		max_result_bytes,
		metrics,
		file_name,
		transaction,
		pending_uncork,
		explicit_transaction,
	);

	// A deferred uncork runs as soon as the transaction it waited on ends.
	if pending_uncork.is_some() && !command_in_tx(db) {
		if let Some(reply) = pending_uncork.take() {
			let _ = reply.send(db.uncork_commits());
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn run_command_inner(
	db: &mut NativeConnection,
	vfs: &NativeVfsHandle,
	command: SqliteCommand,
	max_result_bytes: Option<u64>,
	metrics: Option<&dyn SqliteVfsMetrics>,
	file_name: &str,
	transaction: &mut Option<TransactionTracker>,
	pending_uncork: &mut Option<oneshot::Sender<Result<()>>>,
	explicit_transaction: &watch::Sender<bool>,
) {
	let start = Instant::now();
	match command {
		SqliteCommand::Execute {
			sql,
			params,
			enqueued_at,
			reply,
		} => {
			if reply.is_closed() {
				return;
			}
			begin_transaction_if_needed(db, transaction);
			// Read the transaction state before running so the label reflects the
			// transaction the statement executed against, not the state it leaves
			// behind (a BEGIN flips autocommit off, a COMMIT flips it back on).
			let in_tx = command_in_tx(db);
			let stmt_kind = classify_statement(&sql);
			let worker_result = if let Some(enqueued_at) = enqueued_at {
				let worker_wait_ns = enqueued_at.elapsed().as_nanos() as u64;
				let operation_profile = db.begin_operation_profile();
				let execution_start = Instant::now();
				let result = execute_single_statement_profiled(
					db.as_ptr(),
					&sql,
					params.as_deref(),
					max_result_bytes,
				);
				let execution_ns = execution_start.elapsed().as_nanos() as u64;
				let mut profile = operation_profile.finish();
				profile.worker_wait_ns = worker_wait_ns;
				profile.sqlite_execution_ns = execution_ns.saturating_sub(profile.storage_ns);
				if let Ok((_, query_profile)) = &result {
					profile.bind_count = query_profile.bind_count;
					profile.bind_logical_bytes = query_profile.bind_logical_bytes;
					profile.result_rows = query_profile.result_rows;
					profile.result_columns = query_profile.result_columns;
					profile.result_logical_bytes = query_profile.result_logical_bytes;
				}
				record_command_metrics(
					metrics,
					"execute",
					in_tx,
					stmt_kind,
					&result,
					start.elapsed(),
				);
				SqliteWorkerResult {
					result: result.map(|(value, _)| value),
					profile,
				}
			} else {
				let result = execute_single_statement(
					db.as_ptr(),
					&sql,
					params.as_deref(),
					max_result_bytes,
				);
				record_command_metrics(
					metrics,
					"execute",
					in_tx,
					stmt_kind,
					&result,
					start.elapsed(),
				);
				SqliteWorkerResult {
					result,
					profile: SqliteOperationProfile::default(),
				}
			};
			finalize_transaction_if_complete(db, metrics, file_name, transaction);
			publish_explicit_transaction(db, explicit_transaction);
			let _ = reply.send(worker_command_result(db, vfs, worker_result));
		}
		SqliteCommand::Exec {
			sql,
			enqueued_at,
			reply,
		} => {
			if reply.is_closed() {
				return;
			}
			begin_transaction_if_needed(db, transaction);
			let in_tx = command_in_tx(db);
			let stmt_kind = classify_statement(&sql);
			let worker_result = if let Some(enqueued_at) = enqueued_at {
				let worker_wait_ns = enqueued_at.elapsed().as_nanos() as u64;
				let operation_profile = db.begin_operation_profile();
				let execution_start = Instant::now();
				let result = exec_statements_profiled(db.as_ptr(), &sql, max_result_bytes);
				let execution_ns = execution_start.elapsed().as_nanos() as u64;
				let mut profile = operation_profile.finish();
				profile.worker_wait_ns = worker_wait_ns;
				profile.sqlite_execution_ns = execution_ns.saturating_sub(profile.storage_ns);
				if let Ok((_, query_profile)) = &result {
					profile.bind_count = query_profile.bind_count;
					profile.bind_logical_bytes = query_profile.bind_logical_bytes;
					profile.result_rows = query_profile.result_rows;
					profile.result_columns = query_profile.result_columns;
					profile.result_logical_bytes = query_profile.result_logical_bytes;
				}
				record_command_metrics(metrics, "exec", in_tx, stmt_kind, &result, start.elapsed());
				SqliteWorkerResult {
					result: result.map(|(value, _)| value),
					profile,
				}
			} else {
				let result = exec_statements(db.as_ptr(), &sql, max_result_bytes);
				record_command_metrics(metrics, "exec", in_tx, stmt_kind, &result, start.elapsed());
				SqliteWorkerResult {
					result,
					profile: SqliteOperationProfile::default(),
				}
			};
			finalize_transaction_if_complete(db, metrics, file_name, transaction);
			publish_explicit_transaction(db, explicit_transaction);
			let _ = reply.send(worker_command_result(db, vfs, worker_result));
		}
		SqliteCommand::ExecuteBatch { statements, reply } => {
			if reply.is_closed() {
				return;
			}
			begin_transaction_if_needed(db, transaction);
			if let Some(tracker) = transaction.as_mut() {
				let extra = statements.len().saturating_sub(1) as u64;
				tracker.statement_count = tracker.statement_count.saturating_add(extra);
			}
			let in_tx = command_in_tx(db);
			let stmt_kind = statements
				.first()
				.map_or("other", |(sql, _)| classify_statement(sql));
			let mut results = Vec::with_capacity(statements.len());
			let mut failure = None;
			for (sql, params) in statements {
				match execute_single_statement(
					db.as_ptr(),
					&sql,
					params.as_deref(),
					max_result_bytes,
				) {
					Ok(result) => results.push(result),
					Err(error) => {
						failure = Some(error);
						break;
					}
				}
			}
			let result = match failure {
				Some(error) => Err(error),
				None => Ok(results),
			};
			record_command_metrics(
				metrics,
				"execute_batch",
				in_tx,
				stmt_kind,
				&result,
				start.elapsed(),
			);
			finalize_transaction_if_complete(db, metrics, file_name, transaction);
			publish_explicit_transaction(db, explicit_transaction);
			let worker_result = SqliteWorkerResult {
				result,
				profile: SqliteOperationProfile::default(),
			};
			let _ = reply.send(worker_command_result(db, vfs, worker_result));
		}
		SqliteCommand::Cork { reply } => {
			db.cork_commits();
			let _ = reply.send(Ok(()));
		}
		SqliteCommand::Uncork { reply } => {
			if !command_in_tx(db) {
				let _ = reply.send(db.uncork_commits());
			} else if pending_uncork.is_some() {
				let _ = reply.send(Err(anyhow::anyhow!(
					"an earlier sqlite uncork is still waiting for its transaction"
				)));
			} else {
				*pending_uncork = Some(reply);
			}
		}
		
		
	}
}

fn worker_command_result<T>(
	db: &NativeConnection,
	vfs: &NativeVfsHandle,
	result: SqliteWorkerResult<T>,
) -> Result<SqliteWorkerResult<T>> {
	if let Some(operation) = vfs.take_indeterminate_result() {
		return Err(SqliteCommitIndeterminateError(operation).into());
	}

	// A statement that failed on a storage request that changed nothing, a refusal with a
	// retry hint or a timeout, reports that cause instead of SQLite's generic "disk I/O error".
	let storage_error = vfs.take_storage_error();
	let result = match (storage_error, result) {
		(
			Some(storage_error),
			SqliteWorkerResult {
				result: Err(_),
				profile,
			},
		) => SqliteWorkerResult {
			result: Err(storage_error.into()),
			profile,
		},
		(_, result) => result,
	};

	// A statement that failed because its transaction crossed the commit page cap reports that
	// cause instead of SQLite's generic "database or disk is full".
	match (vfs.take_transaction_too_large(), result) {
		(
			Some(too_large),
			SqliteWorkerResult {
				result: Err(_),
				profile,
			},
		) => {
			finish_refused_rollback(db);
			Ok(SqliteWorkerResult {
				result: Err(too_large.into()),
				profile,
			})
		}
		(_, result) => Ok(result),
	}
}

/// Makes SQLite finish rolling back a transaction refused for its size before the command
/// returns. A refused cache spill leaves SQLite's rollback for its next access to the database,
/// so until then the VFS still holds the abandoned pages. Reading the schema version is that
/// access, and it restores the pages from the journal and commits them, releasing the memory.
fn finish_refused_rollback(db: &NativeConnection) {
	if unsafe { sqlite3_get_autocommit(db.as_ptr()) } == 0 {
		return;
	}
	if let Err(error) = sqlite_exec(db.as_ptr(), "PRAGMA main.schema_version;") {
		tracing::warn!(
			%error,
			"sqlite rollback after a refused oversize transaction failed; it retries on the next access"
		);
	}
}

/// Opens a transaction tracker before running a command when none is active.
///
/// The first command after autocommit resumes starts a new transaction, whether
/// it is an autocommit statement or an explicit `BEGIN`. The round trip baseline
/// is captured here so the delta covers the whole transaction.
fn begin_transaction_if_needed(
	db: &mut NativeConnection,
	transaction: &mut Option<TransactionTracker>,
) {
	match transaction {
		Some(tracker) => tracker.statement_count = tracker.statement_count.saturating_add(1),
		None => {
			*transaction = Some(TransactionTracker {
				started_at: Instant::now(),
				start_round_trips: db.round_trip_counts(),
				statement_count: 1,
			});
		}
	}
}

/// Reports a transaction once the connection returns to autocommit mode.
///
/// Always records the round trip count to the histogram and logs at debug.
/// Transactions slower than `SQLITE_SLOW_TRANSACTION_THRESHOLD` also log at warn.
fn finalize_transaction_if_complete(
	db: &mut NativeConnection,
	metrics: Option<&dyn SqliteVfsMetrics>,
	file_name: &str,
	transaction: &mut Option<TransactionTracker>,
) {
	// Autocommit off means an explicit transaction is still open across the next
	// command, so the transaction has not finished yet.
	if command_in_tx(db) {
		return;
	}
	let Some(tracker) = transaction.take() else {
		return;
	};

	let duration = tracker.started_at.elapsed();
	let round_trips = db.round_trip_counts().since(tracker.start_round_trips);

	if let Some(metrics) = metrics {
		metrics.observe_transaction_round_trips(round_trips.get_pages, round_trips.commit);
	}

	if duration >= SQLITE_SLOW_TRANSACTION_THRESHOLD {
		tracing::warn!(
			database = %file_name,
			duration_ms = duration.as_millis() as u64,
			statement_count = tracker.statement_count,
			get_pages_round_trips = round_trips.get_pages,
			commit_round_trips = round_trips.commit,
			total_round_trips = round_trips.total(),
			"slow sqlite transaction"
		);
	} else {
		tracing::debug!(
			database = %file_name,
			duration_ms = duration.as_millis() as u64,
			statement_count = tracker.statement_count,
			get_pages_round_trips = round_trips.get_pages,
			commit_round_trips = round_trips.commit,
			total_round_trips = round_trips.total(),
			"sqlite transaction complete"
		);
	}
}

fn record_command_metrics<T>(
	metrics: Option<&dyn SqliteVfsMetrics>,
	operation: &'static str,
	in_tx: bool,
	stmt_kind: &'static str,
	result: &Result<T>,
	duration: Duration,
) {
	let Some(metrics) = metrics else {
		return;
	};
	metrics.observe_worker_command_duration(
		operation,
		in_tx,
		stmt_kind,
		duration.as_nanos() as u64,
	);
	if let Err(error) = result {
		metrics.record_worker_command_error(operation, worker_error_code(error));
	}
}

/// Publishes the connection's explicit transaction state, waking subscribers
/// only when it changed.
fn publish_explicit_transaction(
	db: &mut NativeConnection,
	explicit_transaction: &watch::Sender<bool>,
) {
	let open = command_in_tx(db);
	explicit_transaction.send_if_modified(|current| {
		if *current == open {
			return false;
		}
		*current = open;
		true
	});
}

/// Returns whether an explicit transaction is open on the connection.
///
/// `sqlite3_get_autocommit` returns non-zero in autocommit mode (no open
/// transaction) and zero once a `BEGIN` opens one. Implicit per-statement
/// transactions keep autocommit on, so this only reports explicit transactions.
fn command_in_tx(db: &mut NativeConnection) -> bool {
	unsafe { sqlite3_get_autocommit(db.as_ptr()) == 0 }
}

/// Classifies a SQL command by its leading keyword for metric labeling.
///
/// Leading line and block comments are skipped so generated SQL with a header
/// comment still classifies by its first statement. Multi-statement `exec`
/// payloads are labeled by their first statement.
fn classify_statement(sql: &str) -> &'static str {
	let trimmed = strip_leading_sql_noise(sql);
	let keyword: String = trimmed
		.chars()
		.take_while(|c| c.is_ascii_alphabetic())
		.collect();
	match keyword.to_ascii_uppercase().as_str() {
		"SELECT" | "WITH" => "select",
		"INSERT" | "REPLACE" => "insert",
		"UPDATE" => "update",
		"DELETE" => "delete",
		"BEGIN" => "begin",
		"COMMIT" | "END" => "commit",
		"ROLLBACK" => "rollback",
		"SAVEPOINT" => "savepoint",
		"RELEASE" => "release",
		"PRAGMA" => "pragma",
		"CREATE" => "create",
		"DROP" => "drop",
		"ALTER" => "alter",
		"VACUUM" => "vacuum",
		"ANALYZE" => "analyze",
		"ATTACH" => "attach",
		"DETACH" => "detach",
		_ => "other",
	}
}

fn strip_leading_sql_noise(sql: &str) -> &str {
	let mut rest = sql.trim_start();
	loop {
		if let Some(after) = rest.strip_prefix("--") {
			rest = match after.find('\n') {
				Some(idx) => after[idx + 1..].trim_start(),
				None => "",
			};
		} else if let Some(after) = rest.strip_prefix("/*") {
			rest = match after.find("*/") {
				Some(idx) => after[idx + 2..].trim_start(),
				None => "",
			};
		} else {
			return rest;
		}
	}
}

fn fail_command(command: SqliteCommand) {
	// Cancelled requests may sit in the bounded queue until the worker observes
	// them. Once observed during shutdown, they fail instead of running.
	match command {
		SqliteCommand::Execute { reply, .. } => {
			let _ = reply.send(Err(sqlite_closing_error()));
		}
		SqliteCommand::Exec { reply, .. } => {
			let _ = reply.send(Err(sqlite_closing_error()));
		}
		SqliteCommand::ExecuteBatch { reply, .. } => {
			let _ = reply.send(Err(sqlite_closing_error()));
		}
		SqliteCommand::Cork { reply } | SqliteCommand::Uncork { reply } => {
			let _ = reply.send(Err(sqlite_closing_error()));
		}
		
		
	}
}

fn sqlite_closing_error() -> anyhow::Error {
	SqliteWorkerClosingError.into()
}

fn sqlite_worker_dead_error() -> anyhow::Error {
	SqliteWorkerDeadError.into()
}

fn worker_error_code(error: &anyhow::Error) -> &'static str {
	if error
		.downcast_ref::<SqliteWorkerOverloadedError>()
		.is_some()
	{
		"overloaded"
	} else if error.downcast_ref::<SqliteWorkerFatalError>().is_some() {
		"fatal"
	} else if error.downcast_ref::<SqliteWorkerClosingError>().is_some() {
		"closing"
	} else if error.downcast_ref::<SqliteWorkerDeadError>().is_some() {
		"dead"
	} else if error
		.downcast_ref::<SqliteWorkerCloseTimeoutError>()
		.is_some()
	{
		"close_timeout"
	} else {
		"sqlite"
	}
}

#[derive(Debug)]
pub struct SqliteWorkerOverloadedError;

impl fmt::Display for SqliteWorkerOverloadedError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("actor.overloaded: sqlite worker command queue is full")
	}
}

impl Error for SqliteWorkerOverloadedError {}

#[derive(Debug)]
pub struct SqliteWorkerClosingError;

impl fmt::Display for SqliteWorkerClosingError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("sqlite worker is closing")
	}
}

impl Error for SqliteWorkerClosingError {}

#[derive(Debug)]
pub struct SqliteWorkerDeadError;

impl fmt::Display for SqliteWorkerDeadError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("sqlite worker is closed")
	}
}

impl Error for SqliteWorkerDeadError {}

#[derive(Debug)]
pub struct SqliteWorkerCloseTimeoutError;

impl fmt::Display for SqliteWorkerCloseTimeoutError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("sqlite worker close timed out")
	}
}

impl Error for SqliteWorkerCloseTimeoutError {}

#[derive(Debug, Clone)]
pub struct SqliteWorkerFatalError {
	message: String,
}

impl SqliteWorkerFatalError {
	pub fn new(message: String) -> Self {
		Self { message }
	}

	pub fn message(&self) -> &str {
		&self.message
	}
}

impl fmt::Display for SqliteWorkerFatalError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "sqlite worker hit fatal storage error: {}", self.message)
	}
}

impl Error for SqliteWorkerFatalError {}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
	if let Some(message) = payload.downcast_ref::<&str>() {
		message.to_string()
	} else if let Some(message) = payload.downcast_ref::<String>() {
		message.clone()
	} else {
		"unknown panic".to_string()
	}
}








