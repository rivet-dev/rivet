//! Implements `Clock` with `libc::clock_gettime`, `std::time::SystemTime`, and
//! `tokio::time::sleep`. Linux BOOTTIME and Darwin MONOTONIC_RAW include suspend.

#![allow(clippy::disallowed_methods)]
use crate::Clocks;
use crate::{Clock, ClockError};
use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc, OnceLock,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Reads the platform's boot clock in milliseconds.
type BootSource = Box<dyn Fn() -> Result<u64, ClockError> + Send + Sync>;

/// Shared by every clone, without a lock: `last` is the newest published boot reading, and
/// `failure` is the first error, after which every observation returns it.
struct Source {
	last: LastReading,
	failure: OnceLock<ClockError>,
	read: BootSource,
}

/// The newest published boot reading, on its own cache line. Every observer loads it, and
/// a publish would otherwise also invalidate the line `failure` and `read` are loaded from.
#[repr(align(64))]
struct LastReading(AtomicU64);

/// Instances on the same host share boot-time deadlines, including when created later.
/// Never send these deadlines to another host.
#[derive(Clone)]
pub struct TokioClock(Arc<Source>);

impl TokioClock {
	pub fn new() -> Result<Self, ClockError> {
		Self::from_source(Box::new(boot_source_ms))
	}

	fn from_source(read: BootSource) -> Result<Self, ClockError> {
		let first = read()?;
		Ok(Self(Arc::new(Source {
			last: LastReading(AtomicU64::new(first)),
			failure: OnceLock::new(),
			read,
		})))
	}
}

impl Source {
	/// Reads the boot clock and checks it against the newest published reading.
	fn boot_ms(&self) -> Result<u64, ClockError> {
		if let Some(error) = self.failure.get() {
			return Err(error.clone());
		}
		// The source read must not run before this load. The kernel's fast clock path
		// orders its counter read after prior loads (`rdtsc_ordered` on x86, `isb` on
		// arm64); `Acquire` adds nothing on x86 and documents that intent.
		let loaded = self.last.0.load(Ordering::Acquire);
		self.advance(loaded)
	}

	/// Checks a reading taken after `loaded` was loaded. Any value another observer published
	/// before that load was read from the source before this reading, so a monotonic source
	/// never reads below it, even when a newer value has been published since. A lower
	/// reading is a real regression and poisons the clock.
	///
	/// The reading is published only when it is newer than `loaded`. Readings are in
	/// milliseconds, so most observations return the value already published and leave the
	/// shared line unwritten.
	fn advance(&self, loaded: u64) -> Result<u64, ClockError> {
		let current = self.read()?;
		if current < loaded {
			// `previous` is the value this observer loaded; a newer one may have been
			// published since.
			return Err(self.fail(ClockError::MonotonicRegression {
				previous: loaded,
				current,
			}));
		}
		if current > loaded {
			self.last.0.fetch_max(current, Ordering::AcqRel);
		}
		Ok(current)
	}

	fn read(&self) -> Result<u64, ClockError> {
		(self.read)().map_err(|error| self.fail(error))
	}

	/// Records the first failure and returns the one every observation now reports.
	fn fail(&self, error: ClockError) -> ClockError {
		self.failure.get_or_init(|| error).clone()
	}
}

fn boot_source_ms() -> Result<u64, ClockError> {
	#[cfg(any(target_os = "linux", target_os = "android"))]
	let source = libc::CLOCK_BOOTTIME;
	// Darwin MONOTONIC is time-adjusted; RAW uses mach_continuous_time and
	// includes sleep without making lease time depend on wall-clock adjustments.
	#[cfg(target_vendor = "apple")]
	let source = libc::CLOCK_MONOTONIC_RAW;
	#[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
	let source = libc::CLOCK_MONOTONIC;
	let mut ts = std::mem::MaybeUninit::<libc::timespec>::uninit();
	// SAFETY: the OS initializes a timespec on success; it is read only then.
	if unsafe { libc::clock_gettime(source, ts.as_mut_ptr()) } != 0 {
		return Err(ClockError::Source(
			std::io::Error::last_os_error().to_string(),
		));
	}

	let ts = unsafe { ts.assume_init() };
	u64::try_from(ts.tv_sec)
		.ok()
		.and_then(|s| s.checked_mul(1000))
		.and_then(|s| s.checked_add(u64::try_from(ts.tv_nsec).ok()? / 1_000_000))
		.ok_or(ClockError::Overflow)
}

fn unix_ms() -> Result<i64, ClockError> {
	let signed = match SystemTime::now().duration_since(UNIX_EPOCH) {
		Ok(d) => i128::try_from(d.as_millis()).map_err(|_| ClockError::Overflow)?,
		Err(e) => -i128::try_from(e.duration().as_millis()).map_err(|_| ClockError::Overflow)?,
	};
	signed.try_into().map_err(|_| ClockError::Overflow)
}

impl Clock for TokioClock {
	fn observe(&self) -> Result<Clocks, ClockError> {
		let boot_ms = self.0.boot_ms()?;
		let unix_ms = unix_ms().map_err(|error| self.0.fail(error))?;
		Ok(Clocks { boot_ms, unix_ms })
	}

	fn boot_ms(&self) -> Result<u64, ClockError> {
		self.0.boot_ms()
	}

	fn sleep_until(
		&self,
		deadline: u64,
	) -> Pin<Box<dyn Future<Output = Result<(), ClockError>> + Send>> {
		let clock = self.clone();
		Box::pin(async move {
			loop {
				match clock.0.boot_ms() {
					Ok(now) if now < deadline => {
						// Recheck BOOTTIME regularly: Tokio's timers may exclude suspend.
						::tokio::time::sleep(Duration::from_millis((deadline - now).min(100)))
							.await;
					}
					Ok(_) => return Ok(()),
					Err(error) => return Err(error),
				}
			}
		})
	}
}

/// `CLOCK_THREAD_CPUTIME_ID` excludes await/scheduler suspension; pair samples within a single poll.
pub(super) fn thread_cpu_time_ns() -> Result<u64, ClockError> {
	let mut ts = std::mem::MaybeUninit::<libc::timespec>::uninit();
	// SAFETY: clock_gettime initializes the supplied timespec on success, checked before reading.
	if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, ts.as_mut_ptr()) } != 0 {
		return Err(ClockError::Source(
			std::io::Error::last_os_error().to_string(),
		));
	}
	let ts = unsafe { ts.assume_init() };
	u64::try_from(ts.tv_sec)
		.ok()
		.and_then(|s| s.checked_mul(1_000_000_000))
		.and_then(|s| s.checked_add(u64::try_from(ts.tv_nsec).ok()?))
		.ok_or(ClockError::Overflow)
}


