//! A process-wide byte budget for bulk chunk runs (large read-ahead windows such as table scans),
//! so many actors streaming large databases at once leave network and CPU headroom for small,
//! latency-sensitive reads and commits on the same worker. Point reads never wait on it.

use parking_lot::Mutex;

/// A token bucket in bytes, refilled lazily from the boot clock. A caller takes its bytes at
/// once, possibly driving the bucket into debt, and waits until the debt it joined is repaid,
/// so callers start in the order they asked. The debt is capped at `MAX_WAIT_MS` of the rate:
/// a caller that would wait longer waits only that long, so a bulk run never waits for more than
/// about a second before admission and a cancelled caller's bytes are never owed for longer.
/// Under sustained overload the rate can therefore exceed the
/// budget by the capped callers, which keeps bulk reads slow instead of failing them.
/// The longest a bulk run waits for the budget.
pub const MAX_WAIT_MS: u64 = 1_000;

#[derive(Debug)]
pub struct BulkReadBudget {
	bytes_per_sec: u64,
	burst_bytes: u64,
	bucket: Mutex<Bucket>,
}

/// Tokens are counted in thousandths of a byte, so a refill of `elapsed_ms * bytes_per_sec`
/// is exact and no remainder is lost between calls.
#[derive(Debug)]
struct Bucket {
	milli_tokens: i128,
	refilled_boot_ms: u64,
}

impl BulkReadBudget {
	/// A budget of `bytes_per_sec`, starting full at `burst_bytes`.
	pub fn new(bytes_per_sec: u64, burst_bytes: u64) -> Self {
		Self {
			bytes_per_sec: bytes_per_sec.max(1),
			burst_bytes,
			bucket: Mutex::new(Bucket {
				milli_tokens: i128::from(burst_bytes) * 1000,
				refilled_boot_ms: 0,
			}),
		}
	}

	pub fn bytes_per_sec(&self) -> u64 {
		self.bytes_per_sec
	}

	/// Takes `bytes` at `now_boot_ms` and returns the boot time at which the caller may start,
	/// which is `now_boot_ms` when the bucket covered them and at most `MAX_WAIT_MS` later.
	pub fn reserve(&self, bytes: u64, now_boot_ms: u64) -> u64 {
		let mut bucket = self.bucket.lock();
		let rate = i128::from(self.bytes_per_sec);
		let elapsed_ms = now_boot_ms.saturating_sub(bucket.refilled_boot_ms);
		let burst = i128::from(self.burst_bytes) * 1000;
		bucket.milli_tokens = (bucket.milli_tokens + i128::from(elapsed_ms) * rate).min(burst);
		bucket.refilled_boot_ms = now_boot_ms.max(bucket.refilled_boot_ms);
		bucket.milli_tokens -= i128::from(bytes) * 1000;
		if bucket.milli_tokens >= 0 {
			return now_boot_ms;
		}

		// The debt is repaid at the budget's rate, a millisecond's worth per millisecond. A
		// caller past the cap waits the cap, and the debt it adds beyond the cap is forgiven.
		let max_debt = rate * i128::from(MAX_WAIT_MS);
		bucket.milli_tokens = bucket.milli_tokens.max(-max_debt);
		let debt = -bucket.milli_tokens;
		let wait_ms = (debt + rate - 1) / rate;
		now_boot_ms.saturating_add(u64::try_from(wait_ms).unwrap_or(MAX_WAIT_MS))
	}
}
