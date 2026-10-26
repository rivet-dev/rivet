use serde::{Deserialize, Serialize};

/// Millisecond budgets shared by all members. Store timestamps may be truncated
/// to seconds; takeover waits beyond both the lease and the store uncertainty.
///
/// One renewal with the default values. `0` is when the renewal PUT is issued; the owner
/// stops serving before readers expire it, and a same-ID successor waits longer still.
///
/// ```text
/// seconds          0  3          13.5                         27  30  32
///                  |--|-----------|----------------------------|---|---|
/// renew timer      |<>| renew_interval_ms
/// owner admits new |<------------>| degrade_after_ms: new tablets and volume opens
/// owner serves     |<----------------------------------------->| ttl_ms - safety_margin_ms
/// readers see live |<--------------------------------------------->| ttl_ms after Last-Modified
/// same-ID takeover |<------------------------------------------------->| ttl_ms + zombie_grace_ms
/// ```
///
/// The defaults are sized for object-store tails. Whole-account stalls hold small requests
/// for about 10 s a few times a month, so the lease must outlast two such requests in a row:
/// each renewal compares-and-swaps on the previous one's ETag, so a sustained per-request
/// latency `L` survives while `2 * L` fits in the serving window, about 13 s here. A crashed
/// worker is therefore detected only after `ttl_ms` plus discovery, about 31 to 33 s.
///
/// The 3 s `safety_margin_ms` separates the owner's last serving instant from the first
/// instant any reader may judge the heartbeat expired. A reader measures age from
/// `Last-Modified`, which can be truncated by up to 1 s, so it may judge expiry 1 s early;
/// the remaining 2 s covers boot-clock rate error over the 30 s window and a reply admitted
/// just before the deadline. Readers hand the worker's tablets to another worker as soon as
/// they judge it expired, so this margin is the only gap between two owners. A reader judges
/// expiry only from a listing issued after the heartbeat would have expired: a renewal that
/// lands before that listing shows in it, and one that lands later is confirmed only after
/// the owner's previous deadline, by which time the owner has fenced.
///
/// Rolling out a change to `ttl_ms` or `safety_margin_ms` is stop-the-world: every worker of a
/// cluster must run the same values. A reader running a shorter `ttl_ms` would judge a peer
/// expired, and re-grant its tablets, while the peer still serves under its longer window. A
/// rolling deploy would have to first raise the `ttl_ms` every reader uses, and only then the
/// window each owner serves.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeaseTiming {
	/// Heartbeat lifetime measured in store time from `Last-Modified`.
	pub ttl_ms: u64,
	/// `Timer::Renew` is scheduled from the latest send time, not its response time.
	pub renew_interval_ms: u64,
	/// The owner stops serving this long before `ttl_ms` elapses.
	pub safety_margin_ms: u64,
	/// Extra wait beyond `ttl_ms` for an unchanged Active or Draining heartbeat; Released bypasses it.
	pub zombie_grace_ms: u64,
	/// Executor I/O and pending-read budget; also the delay for `Timer::Retry`.
	pub io_timeout_ms: u64,
	/// How long a live lease's renewal PUT or readback GET waits before another attempt
	/// starts, never past the current lease deadline. A renewal PUT that reaches this budget is
	/// not abandoned: it keeps running until the lease deadline beside a fresh CAS write on the
	/// same ETag, and whichever answers first confirms its own write at its own send time. At
	/// most one of them can land. Equal to `renew_interval_ms` by default, so a stalled store
	/// sees heartbeat writes start at the normal renewal rate, never faster.
	#[serde(default = "default_heartbeat_attempt_ms")]
	pub heartbeat_attempt_ms: u64,
	/// Longest delay after a listing response before the next listing. A listing also
	/// starts when the view would expire, and early on a hint; see `refresh_hint_interval_ms`.
	pub refresh_interval_ms: u64,
	/// Minimum spacing between listings started by `Event::Refresh` or by a point read
	/// that found a storage worker missing from discovery.
	#[serde(default = "default_refresh_hint_interval_ms")]
	pub refresh_hint_interval_ms: u64,
	/// Registration budget before fencing `RegistrationFailed`.
	pub boot_timeout_ms: u64,
	/// Bound on `Last-Modified` error, which can be truncated to seconds. Validation requires
	/// `zombie_grace_ms + safety_margin_ms` to cover this bound plus 1,000 ms.
	pub store_timestamp_uncertainty_ms: u64,
}

impl Default for LeaseTiming {
	fn default() -> Self {
		Self {
			ttl_ms: 30_000,
			renew_interval_ms: 3_000,
			safety_margin_ms: 3_000,
			zombie_grace_ms: 2_000,
			io_timeout_ms: 1_000,
			heartbeat_attempt_ms: default_heartbeat_attempt_ms(),
			refresh_interval_ms: 10_000,
			refresh_hint_interval_ms: default_refresh_hint_interval_ms(),
			boot_timeout_ms: 60_000,
			store_timestamp_uncertainty_ms: 1_000,
		}
	}
}

fn default_refresh_hint_interval_ms() -> u64 {
	1_000
}

fn default_heartbeat_attempt_ms() -> u64 {
	3_000
}

impl LeaseTiming {
	/// How long a routing hint taken from one heartbeat stays usable: until that heartbeat's
	/// lease would end, minus the safety margin, but never below `floor_ms` and never past
	/// `ttl_ms`. `modified_unix_ms` is the heartbeat's `Last-Modified` and `store_now_unix_ms`
	/// a lower bound of store time. A lower bound can only overstate the remaining lease, which
	/// is acceptable for a hint: serving still checks the owner's latest lease at admission,
	/// and a failed request invalidates the hint.
	pub fn hint_ttl_ms(&self, modified_unix_ms: i64, store_now_unix_ms: i64, floor_ms: u64) -> u64 {
		let lease_end_unix_ms =
			modified_unix_ms as i128 + self.ttl_ms as i128 - self.safety_margin_ms as i128;
		let remaining_ms = (lease_end_unix_ms - store_now_unix_ms as i128).max(0) as u128;
		let remaining_ms = remaining_ms.min(self.ttl_ms as u128) as u64;
		remaining_ms.max(floor_ms.min(self.ttl_ms))
	}

	/// How long the owner may serve after a confirmed renewal's send time.
	pub fn serving_ms(&self) -> u64 {
		self.ttl_ms.saturating_sub(self.safety_margin_ms)
	}

	/// How long after a confirmed renewal's send time the owner still takes on new ownership:
	/// tablet grants, replica placements and volume opens. Past it, until the next confirmed
	/// renewal, the worker serves only what it already holds, so a store stall that later
	/// fences it strands as little as possible.
	pub fn degrade_after_ms(&self) -> u64 {
		self.serving_ms() / 2
	}

	pub fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.ttl_ms
				> self
					.safety_margin_ms
					.saturating_add(self.renew_interval_ms)
					.saturating_add(self.io_timeout_ms),
			"heartbeat TTL must cover renewal, timeout, and margin"
		);
		anyhow::ensure!(
			self.ttl_ms
				> self
					.safety_margin_ms
					.saturating_add(self.renew_interval_ms)
					.saturating_add(self.heartbeat_attempt_ms),
			"heartbeat TTL must cover renewal, one renewal attempt, and margin"
		);
		anyhow::ensure!(
			self.zombie_grace_ms.saturating_add(self.safety_margin_ms)
				>= 1_000u64.saturating_add(self.store_timestamp_uncertainty_ms),
			"takeover must cover store timestamp uncertainty"
		);
		anyhow::ensure!(
			self.boot_timeout_ms
				> self
					.ttl_ms
					.saturating_add(self.zombie_grace_ms)
					.saturating_add(self.store_timestamp_uncertainty_ms)
					.saturating_add(self.safety_margin_ms),
			"registration must outlast a same-ID takeover wait"
		);
		anyhow::ensure!(
			[
				self.ttl_ms,
				self.renew_interval_ms,
				self.io_timeout_ms,
				self.heartbeat_attempt_ms,
				self.refresh_interval_ms,
				self.refresh_hint_interval_ms,
				self.boot_timeout_ms
			]
			.into_iter()
			.all(|v| v > 0),
			"membership intervals must be positive"
		);

		Ok(())
	}
}

/// Validated by topology construction before any I/O starts.
/// Placement settings must agree cluster-wide; local lookup limits bound memory and concurrent reads.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopologyConfig {
	/// Shared by the state machine's deadlines and the executor's I/O timeout.
	pub lease: LeaseTiming,
	/// Whole storage listing budget, separate from one heartbeat read's I/O budget.
	#[serde(default = "default_discovery_timeout_ms")]
	pub discovery_timeout_ms: u64,
	/// Endpoint cache cap. Reaching it evicts the soonest-expiring entry.
	pub endpoint_capacity: usize,
	/// Concurrent endpoint and inspection reads, 1 to 64.
	pub lookup_concurrency: usize,
	/// Lifetime of a cached miss, and the shortest lifetime of a cached endpoint. A found
	/// endpoint is cached until its heartbeat's lease would end (`LeaseTiming::hint_ttl_ms`).
	pub lookup_ttl_ms: u64,
	/// Concurrent lookup and heartbeat inspection requests admitted into the event loop.
	/// Requests above this fail immediately instead of queueing.
	pub lookup_request_capacity: usize,
	/// An opening timeout to a stateless worker drops its cached endpoint only when a point
	/// read last confirmed that endpoint at least this long ago. A timeout under overload is
	/// not evidence of a stale route, so recently confirmed routes are kept.
	pub endpoint_timeout_invalidate_after_ms: u64,
	/// Timeout reports waiting in the topology inbox. Reports above this are dropped, so
	/// they never crowd out endpoint resolution.
	pub endpoint_timeout_report_capacity: usize,
	/// Endpoint invalidations admitted into the event loop, separate from lookup admission.
	/// A failed connection reports the same endpoint once per failed request, and the state
	/// machine ignores repeats, so invalidations above this are dropped instead of taking
	/// permits that endpoint resolution needs.
	pub invalidation_request_capacity: usize,
	/// Cache and admission of exact-id lease status reads.
	pub worker_lease: crate::worker_lease::WorkerLeaseConfig,
	/// Process watches; see `Topology::watch`.
	#[serde(default)]
	pub watch: WatchConfig,
}

/// Bounds for `Topology::watch`. A watch exists for low-urgency cleanup after a crashed
/// process, such as unloading the volumes it left open, and never for fencing or anything
/// correctness-critical, so its detection is deliberately slow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct WatchConfig {
	/// How often each watched stateless process's heartbeat is read, once per process however
	/// many subscribers watch it. Minutes, on purpose: a watch only lets cleanup start, so a
	/// short interval would buy nothing but object-store reads. Storage workers are judged by
	/// the ordinary discovery listing instead and cost no reads.
	pub poll_interval_ms: u64,
	/// Watched processes. `Topology::watch` refuses a new process at this cap.
	pub capacity: usize,
	/// Poll reads in flight. A due poll above it reads nothing and waits for its next interval.
	pub poll_concurrency: usize,
}

impl Default for WatchConfig {
	fn default() -> Self {
		Self {
			poll_interval_ms: 120_000,
			capacity: 16_384,
			poll_concurrency: 8,
		}
	}
}

impl Default for TopologyConfig {
	fn default() -> Self {
		Self {
			lease: LeaseTiming::default(),
			discovery_timeout_ms: default_discovery_timeout_ms(),
			endpoint_capacity: 4096,
			lookup_concurrency: 32,
			lookup_ttl_ms: 2_000,
			lookup_request_capacity: 128,
			endpoint_timeout_invalidate_after_ms: 5_000,
			endpoint_timeout_report_capacity: 64,
			invalidation_request_capacity: 64,
			worker_lease: Default::default(),
			watch: Default::default(),
		}
	}
}

fn default_discovery_timeout_ms() -> u64 {
	5_000
}

impl TopologyConfig {
	pub fn validate(&self) -> anyhow::Result<()> {
		self.lease.validate()?;
		anyhow::ensure!(
			self.discovery_timeout_ms > 0,
			"discovery timeout must be positive"
		);
		anyhow::ensure!(
			self.endpoint_capacity > 0 && self.endpoint_capacity <= i64::MAX as usize,
			"endpoint capacity must be positive and fit metrics"
		);
		anyhow::ensure!(
			(1..=64).contains(&self.lookup_concurrency),
			"lookup concurrency must be 1..=64"
		);
		anyhow::ensure!(self.lookup_ttl_ms > 0, "lookup TTL must be positive");
		anyhow::ensure!(
			self.lookup_request_capacity > 0 && self.lookup_request_capacity <= i64::MAX as usize,
			"lookup request capacity must be positive and fit metrics"
		);
		anyhow::ensure!(
			self.endpoint_timeout_report_capacity > 0,
			"endpoint timeout report capacity must be positive"
		);
		anyhow::ensure!(
			self.invalidation_request_capacity > 0,
			"endpoint invalidation request capacity must be positive"
		);
		anyhow::ensure!(
			self.worker_lease.cache_capacity > 0
				&& self.worker_lease.cache_capacity <= i64::MAX as u64
				&& self.worker_lease.request_capacity > 0
				&& self.worker_lease.terminal_ttl_ms > 0,
			"worker lease cache, admission and TTL must be positive"
		);
		anyhow::ensure!(
			self.watch.poll_interval_ms >= self.lease.io_timeout_ms,
			"watch poll interval must cover a heartbeat read; watches are for slow cleanup"
		);
		anyhow::ensure!(
			self.watch.capacity > 0
				&& self.watch.capacity <= i64::MAX as usize
				&& self.watch.poll_concurrency > 0
				&& self.watch.poll_concurrency <= tokio::sync::Semaphore::MAX_PERMITS,
			"watch capacity and poll concurrency must be positive"
		);

		Ok(())
	}
}
