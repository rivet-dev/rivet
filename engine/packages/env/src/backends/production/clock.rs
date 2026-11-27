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
			let mut suspends = None;
			loop {
				match clock.0.boot_ms() {
					Ok(now) if now < deadline => {
						let remaining = Duration::from_millis(deadline - now);
						// Tokio's timers may exclude suspend, so after a host suspend a sleeper
						// would wake late by up to its whole remaining time. A short sleep is
						// bounded by that time anyway. A long one also wakes when the suspend
						// watcher sees boot time jump ahead of the timer clock, and rechecks.
						// The watcher is one thread for the process; a recheck timer per
						// sleeper woke every SlateDB ticker, poll and lease ten times a second
						// (about 1,500 timer polls a second) on an idle engine.
						if remaining <= SHORT_SLEEP {
							::tokio::time::sleep(remaining).await;
						} else {
							let Some(events) = suspends.as_mut() else {
								// Subscribe, then read the clock again: a suspend between the
								// first reading and the subscription must not be missed.
								suspends = Some(suspend::subscribe());
								continue;
							};
							match events {
								Some(events) => {
									crate::select_biased!("either ends the wait and the loop rereads the boot clock",
										() = ::tokio::time::sleep(remaining) => {},
										() = suspend::next(events) => {},
									)
								}
								// No watcher: recheck the boot clock every 100 ms.
								None => {
									::tokio::time::sleep(remaining.min(Duration::from_millis(100)))
										.await
								}
							}
						}
					}
					Ok(_) => return Ok(()),
					Err(error) => return Err(error),
				}
			}
		})
	}
}

/// Sleeps this short are not worth a subscription to the suspend watcher.
const SHORT_SLEEP: Duration = Duration::from_millis(250);

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

/// Host suspend detection for [`TokioClock::sleep_until`].
///
/// The boot clock (`CLOCK_BOOTTIME`, Darwin `CLOCK_MONOTONIC_RAW`) counts time the host spent
/// suspended; the clock Tokio's timers run on (`CLOCK_MONOTONIC`, Darwin `CLOCK_UPTIME_RAW`)
/// does not. A suspend therefore shows as the boot clock jumping ahead of the timer clock.
/// One thread per process samples both every [`SAMPLE`] and publishes each jump, so sleepers
/// wake and recheck the boot clock within about one sample of the host resuming, whatever
/// their own remaining time. Where the two clocks are the same clock there is nothing to
/// detect and no thread runs.
mod suspend {
	use std::{sync::OnceLock, time::Duration};

	use tokio::sync::watch;

	/// How often the watcher compares the two clocks.
	const SAMPLE: Duration = Duration::from_millis(100);

	/// Growth of (boot clock - timer clock) between two samples that counts as a suspend. A
	/// sample lands within a few milliseconds of its time, so ordinary scheduling jitter stays
	/// far below it.
	const JUMP_MS: i128 = 50;

	static SUSPENDS: OnceLock<Option<watch::Sender<u64>>> = OnceLock::new();

	/// A receiver for suspend notices, or `None` when the watcher thread could not be started.
	/// Callers then recheck the boot clock on a short interval themselves.
	pub(super) fn subscribe() -> Option<watch::Receiver<u64>> {
		SUSPENDS
			.get_or_init(|| {
				start(|run| {
					std::thread::Builder::new()
						.name("rivet-suspend-watch".into())
						.spawn(run)
						.map(|_| ())
				})
			})
			.as_ref()
			.map(watch::Sender::subscribe)
	}

	/// Completes when a suspend was detected after the receiver last looked. Never completes
	/// if the watcher is gone.
	pub(super) async fn next(events: &mut watch::Receiver<u64>) {
		if events.changed().await.is_err() {
			std::future::pending::<()>().await;
		}
	}

	/// Whether the gap between the boot clock and the timer clock grew by a suspend's worth.
	pub(super) fn suspended(previous_gap_ms: i128, gap_ms: i128) -> bool {
		gap_ms - previous_gap_ms >= JUMP_MS
	}

	fn start(
		spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> std::io::Result<()>,
	) -> Option<watch::Sender<u64>> {
		let (sender, _receiver) = watch::channel(0u64);
		if let Some(first) = gap_ms() {
			let sender = sender.clone();
			let run = move || {
				let mut previous = first;
				loop {
					std::thread::sleep(SAMPLE);
					let Some(gap) = gap_ms() else {
						continue;
					};
					if suspended(previous, gap) {
						sender.send_modify(|count| *count += 1);
					}
					previous = gap;
				}
			};
			if let Err(error) = spawn(Box::new(run)) {
				tracing::warn!(%error, "could not start the suspend watcher; long sleeps recheck the boot clock every 100 ms");
				return None;
			}
		}
		Some(sender)
	}

	/// Boot clock minus timer clock in milliseconds, or `None` where they are one clock.
	fn gap_ms() -> Option<i128> {
		let timer = timer_source_ms()?;
		let boot = i128::from(super::boot_source_ms().ok()?);
		Some(boot - i128::from(timer))
	}

	#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
	fn timer_source_ms() -> Option<u64> {
		#[cfg(any(target_os = "linux", target_os = "android"))]
		let source = libc::CLOCK_MONOTONIC;
		#[cfg(target_vendor = "apple")]
		let source = libc::CLOCK_UPTIME_RAW;
		let mut ts = std::mem::MaybeUninit::<libc::timespec>::uninit();
		// SAFETY: the OS initializes a timespec on success; it is read only then.
		if unsafe { libc::clock_gettime(source, ts.as_mut_ptr()) } != 0 {
			return None;
		}
		let ts = unsafe { ts.assume_init() };
		u64::try_from(ts.tv_sec)
			.ok()?
			.checked_mul(1000)?
			.checked_add(u64::try_from(ts.tv_nsec).ok()? / 1_000_000)
	}

	#[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
	fn timer_source_ms() -> Option<u64> {
		None
	}

	#[cfg(test)]
	mod tests {
		use super::{JUMP_MS, start, suspended};

		#[test]
		fn no_watcher_when_the_thread_cannot_be_spawned() {
			let failing = |_run: Box<dyn FnOnce() + Send>| Err(std::io::Error::other("no threads"));
			// On platforms where the clocks are one clock no thread is needed and `start`
			// succeeds without spawning.
			let sender = start(failing);
			assert_eq!(sender.is_none(), super::gap_ms().is_some());
		}

		#[test]
		fn a_jump_of_a_suspend_is_detected_and_jitter_is_not() {
			assert!(!suspended(1_000, 1_000));
			assert!(!suspended(1_000, 1_000 + JUMP_MS - 1));
			assert!(suspended(1_000, 1_000 + JUMP_MS));
			assert!(suspended(1_000, 1_000 + 60_000));
			// The gap never shrinks on a healthy host; if it does, that is no suspend.
			assert!(!suspended(1_000, 900));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn a_long_sleep_ends_at_its_deadline() {
		let clock = TokioClock::new().expect("clock");
		let start = clock.boot_ms().expect("boot");
		clock.sleep_until(start + 600).await.expect("sleep");
		let end = clock.boot_ms().expect("boot");
		assert!(
			(600..900).contains(&(end - start)),
			"slept {} ms",
			end - start
		);
	}

	#[tokio::test]
	async fn a_passed_deadline_returns_at_once() {
		let clock = TokioClock::new().expect("clock");
		let now = clock.boot_ms().expect("boot");
		clock
			.sleep_until(now.saturating_sub(5))
			.await
			.expect("sleep");
	}
}
