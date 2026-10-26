//! jemalloc global allocator.
//!
//! The engine frees most memory on a different thread than allocated it, since requests, replies
//! and task state cross worker threads through channels. glibc malloc then takes the owning
//! arena's lock on every such free, and under load busy Tokio workers spend a large share of
//! their time parked on those locks. jemalloc frees through per-thread caches instead.
//!
//! jemalloc's resident bytes are exported as `rivet_process_allocator_bytes{kind="committed"}`,
//! and its cumulative allocation requests as `rivet_process_allocations_total`. Dividing that
//! counter's rate by an operation rate, such as actor generations stopped, gives allocations per
//! operation under steady load.
//! Builds with the `heap-profile` feature also sample heap profiles and write one on SIGUSR2,
//! so load runs can name the allocation sites that hold memory.

use prometheus::{
	IntCounter, IntGaugeVec, Opts,
	core::{Collector, Desc},
	proto::MetricFamily,
};
use std::{
	ffi::{CStr, c_void},
	sync::atomic::{AtomicU64, Ordering},
};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Advances jemalloc's statistics epoch, so the next reads see current values.
fn refresh_stats() -> Option<()> {
	let mut epoch: u64 = 1;
	let mut epoch_len = std::mem::size_of::<u64>();
	// SAFETY: `epoch` is a `uint64_t` control; both pointers refer to a live `u64`.
	let result = unsafe {
		tikv_jemalloc_sys::mallctl(
			c"epoch".as_ptr(),
			(&raw mut epoch).cast::<c_void>(),
			&mut epoch_len,
			(&raw mut epoch).cast::<c_void>(),
			std::mem::size_of::<u64>(),
		)
	};
	if result == 0 { Some(()) } else { None }
}

/// Reads one `size_t` or `uint64_t` statistic. Both are 64 bits on supported targets.
fn read_stat(name: &CStr) -> Option<u64> {
	let mut value: u64 = 0;
	let mut len = std::mem::size_of::<u64>();
	// SAFETY: the named controls are 64-bit integers, read into a live `u64` of that size.
	let result = unsafe {
		tikv_jemalloc_sys::mallctl(
			name.as_ptr(),
			(&raw mut value).cast::<c_void>(),
			&mut len,
			std::ptr::null_mut(),
			0,
		)
	};
	if result == 0 && len == std::mem::size_of::<u64>() {
		Some(value)
	} else {
		None
	}
}

/// Memory jemalloc holds from the OS.
pub fn committed_bytes() -> Option<i64> {
	refresh_stats()?;
	i64::try_from(read_stat(c"stats.resident")?).ok()
}

/// Allocation requests served since the process started, summed over every arena: small ones
/// (including those served from thread caches, as of the caches' last merge into their arena)
/// and large ones. Call after `refresh_stats`.
fn allocation_requests() -> Option<u64> {
	// 4096 is `MALLCTL_ARENAS_ALL`, the merged statistics of every arena.
	let small = read_stat(c"stats.arenas.4096.small.nrequests")?;
	let large = read_stat(c"stats.arenas.4096.large.nrequests")?;
	Some(small + large)
}

/// Reads the allocator at scrape time, so the metrics need no update loop.
struct AllocatorCollector {
	bytes: IntGaugeVec,
	allocations: IntCounter,
	/// The allocation count already added to `allocations`.
	counted: AtomicU64,
}

impl Collector for AllocatorCollector {
	fn desc(&self) -> Vec<&Desc> {
		let mut descs = self.bytes.desc();
		descs.extend(self.allocations.desc());
		descs
	}

	fn collect(&self) -> Vec<MetricFamily> {
		if let Some(committed) = committed_bytes() {
			self.bytes.with_label_values(&["committed"]).set(committed);
		}
		if let Some(requests) = allocation_requests() {
			// Thread caches merge their counts late, so a read can trail an earlier one.
			let previous = self.counted.fetch_max(requests, Ordering::Relaxed);
			self.allocations.inc_by(requests.saturating_sub(previous));
		}
		let mut families = self.bytes.collect();
		families.extend(self.allocations.collect());
		families
	}
}

/// Registers `rivet_process_allocator_bytes` and `rivet_process_allocations_total` once per
/// process.
pub fn register_metrics() -> anyhow::Result<()> {
	let bytes = IntGaugeVec::new(
		Opts::new(
			"process_allocator_bytes",
			"Allocator memory held from the OS (committed).",
		),
		&["kind"],
	)?;
	let allocations = IntCounter::with_opts(Opts::new(
		"process_allocations_total",
		"Allocation requests the allocator served, read from jemalloc at scrape time.",
	))?;
	rivet_metrics::REGISTRY.register(Box::new(AllocatorCollector {
		bytes,
		allocations,
		counted: AtomicU64::new(0),
	}))?;
	Ok(())
}

#[cfg(feature = "heap-profile")]
mod profile {
	//! jemalloc heap profiling: compile-time options and the SIGUSR2 dump trigger.

	use anyhow::{Context, Result};
	use std::{
		ffi::{CString, c_char, c_void},
		path::PathBuf,
	};

	#[repr(transparent)]
	pub struct MallocConf(*const c_char);

	// The pointer refers to a static C string that is never written.
	unsafe impl Sync for MallocConf {}

	/// One sample per 512 KiB allocated on average. `_RJEM_MALLOC_CONF` can override it at
	/// startup.
	#[unsafe(export_name = "_rjem_malloc_conf")]
	pub static MALLOC_CONF: MallocConf =
		MallocConf(c"prof:true,prof_active:true,lg_prof_sample:19".as_ptr());

	/// Directory for dumps, from `RIVET_HEAP_PROFILE_DIR`, default `/tmp`.
	fn dump_dir() -> PathBuf {
		std::env::var_os("RIVET_HEAP_PROFILE_DIR")
			.map(PathBuf::from)
			.unwrap_or_else(|| PathBuf::from("/tmp"))
	}

	pub(super) fn dump(sequence: u64) -> Result<PathBuf> {
		let path = dump_dir().join(format!("rivet-heap.{}.{sequence}.heap", std::process::id()));
		let name = CString::new(path.as_os_str().as_encoded_bytes())
			.context("heap profile path contains a NUL byte")?;
		let mut filename: *const c_char = name.as_ptr();
		let result = unsafe {
			tikv_jemalloc_sys::mallctl(
				c"prof.dump".as_ptr(),
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				(&raw mut filename).cast::<c_void>(),
				std::mem::size_of::<*const c_char>(),
			)
		};
		anyhow::ensure!(result == 0, "prof.dump failed with error {result}");
		Ok(path)
	}

	/// Writes a heap profile on every SIGUSR2 until the scope shuts down.
	pub fn listen(scope: &rivet_env::TaskScope) -> Result<()> {
		use tokio::signal::unix::{SignalKind, signal};

		let mut signals = signal(SignalKind::user_defined2())?;
		scope.spawn(async move {
			let mut sequence = 0u64;
			while signals.recv().await.is_some() {
				match dump(sequence) {
					Ok(path) => tracing::info!(path = %path.display(), "wrote heap profile"),
					Err(error) => tracing::warn!(?error, "heap profile dump failed"),
				}
				sequence += 1;
			}
		});
		Ok(())
	}
}

/// Starts the heap profile dump trigger.
#[cfg(feature = "heap-profile")]
pub fn listen_for_heap_dumps(scope: &rivet_env::TaskScope) -> anyhow::Result<()> {
	profile::listen(scope)?;
	tracing::info!("heap profiling enabled; send SIGUSR2 to write a profile");
	Ok(())
}


