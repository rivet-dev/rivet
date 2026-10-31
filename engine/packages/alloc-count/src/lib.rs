//! A counting `std::alloc::GlobalAlloc` wrapper over `std::alloc::System` for allocation-count
//! tests.
//!
//! A test binary installs [`CountingAllocator`] as its `#[global_allocator]` and wraps the code
//! under test in [`count`]. Counting is per thread, so tests that run in parallel threads of the
//! same binary do not see each other's allocations. Code that allocates on another thread (a
//! spawned task on a multi-threaded runtime) is not counted.
//!
//! [`process_totals`] counts every allocation in the process on every thread, for benchmarks
//! that divide a whole-process delta by the operations they ran.

#[cfg(feature = "sites")]
pub mod sites;

use std::{
	alloc::{GlobalAlloc, Layout, System},
	cell::Cell,
	sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

/// Allocations made on the current thread while [`count`] runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Allocations {
	/// Calls to `alloc`, `alloc_zeroed` and `realloc`.
	pub count: u64,
	/// Bytes requested by those calls. A `realloc` counts its new size.
	pub bytes: u64,
	/// Bytes this thread freed while counting, including memory allocated before. A `realloc`
	/// counts its old size. `bytes - freed_bytes` approximates what the counted code retained.
	pub freed_bytes: u64,
}

thread_local! {
	static ACTIVE: Cell<bool> = const { Cell::new(false) };
	static COUNT: Cell<u64> = const { Cell::new(0) };
	static BYTES: Cell<u64> = const { Cell::new(0) };
	static FREED: Cell<u64> = const { Cell::new(0) };
}

fn record_free(size: usize) {
	let active = ACTIVE.try_with(Cell::get).unwrap_or(false);
	if active {
		let _ = FREED.try_with(|freed| freed.set(freed.get() + size as u64));
	}
}

/// Process-wide counter slots. Threads spread over the slots so that counting does not make
/// every allocation contend on one cache line.
const SLOTS: usize = 64;

#[repr(align(128))]
struct Slot {
	count: AtomicU64,
	bytes: AtomicU64,
}

static PROCESS: [Slot; SLOTS] = [const {
	Slot {
		count: AtomicU64::new(0),
		bytes: AtomicU64::new(0),
	}
}; SLOTS];

static NEXT_SLOT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
	static SLOT: usize = NEXT_SLOT.fetch_add(1, Ordering::Relaxed) % SLOTS;
}

fn record_process(size: usize) {
	// Thread teardown has no slot left; those allocations land in slot 0.
	let slot = SLOT.try_with(|slot| *slot).unwrap_or(0);
	let _sequence = PROCESS[slot].count.fetch_add(1, Ordering::Relaxed);
	#[cfg(feature = "sites")]
	sites::maybe_sample(_sequence, size);
	PROCESS[slot]
		.bytes
		.fetch_add(size as u64, Ordering::Relaxed);
}

/// Allocations made by every thread of the process since it started: the `count` and `bytes`
/// fields of [`Allocations`], with `freed_bytes` left at zero. Subtract two snapshots to count
/// an interval.
pub fn process_totals() -> Allocations {
	let mut totals = Allocations::default();
	for slot in &PROCESS {
		totals.count += slot.count.load(Ordering::Relaxed);
		totals.bytes += slot.bytes.load(Ordering::Relaxed);
	}
	totals
}

/// Counts allocations made by this thread while [`count`] is running, and every allocation in
/// the process for [`process_totals`].
pub struct CountingAllocator;

fn record(size: usize) {
	record_process(size);
	// `try_with` fails only during thread teardown, when nothing is being counted.
	let active = ACTIVE.try_with(Cell::get).unwrap_or(false);
	if active {
		let _ = COUNT.try_with(|count| count.set(count.get() + 1));
		let _ = BYTES.try_with(|bytes| bytes.set(bytes.get() + size as u64));
	}
}

unsafe impl GlobalAlloc for CountingAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		record(layout.size());
		unsafe { System.alloc(layout) }
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		record(layout.size());
		unsafe { System.alloc_zeroed(layout) }
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		record_free(layout.size());
		unsafe { System.dealloc(ptr, layout) }
	}

	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		record(new_size);
		record_free(layout.size());
		unsafe { System.realloc(ptr, layout, new_size) }
	}
}

/// Runs `f` and returns its result with the allocations it made on this thread. Calls do not
/// nest: an inner call's allocations are not added to the outer one.
pub fn count<T>(f: impl FnOnce() -> T) -> (T, Allocations) {
	COUNT.with(|count| count.set(0));
	BYTES.with(|bytes| bytes.set(0));
	FREED.with(|freed| freed.set(0));
	ACTIVE.with(|active| active.set(true));
	let value = f();
	ACTIVE.with(|active| active.set(false));
	let allocations = Allocations {
		count: COUNT.with(Cell::get),
		bytes: BYTES.with(Cell::get),
		freed_bytes: FREED.with(Cell::get),
	};
	(value, allocations)
}
