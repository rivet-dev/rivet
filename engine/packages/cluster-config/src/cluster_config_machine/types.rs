//! Config protocol values; persisted documents retain the existing generated v1 BARE encoding.
use rivet_state_machine_util::IoTimer;
use serde::{Deserialize, Serialize};

/// One serialized I/O lane shared with the state machine core.
pub use rivet_state_machine::SingleSlot as Slot;

/// Parent-lifetime correlation ID; never reused when replacing a conditional-write helper.
pub type OpId = rivet_state_machine::OpId<Slot>;

/// Dispatch deadline projected from the current I/O phase.
pub use rivet_state_machine_util::IoCall;

/// One authoritative S3 value; the ETag is a condition, not a config generation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoredConfig {
	/// Exact generated vbare document returned by S3.
	pub bytes: Vec<u8>,
	/// Required for safe conditional updates and unchanged-refresh detection.
	pub etag: String,
}
/// Immutable conditional request, retained through unknown outcomes and retries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PreparedConfig {
	/// Validated generated vbare document; repeat attempts send identical bytes.
	pub bytes: Vec<u8>,
	/// None creates the initial object. Some replaces exactly the read version.
	pub expected_etag: Option<String>,
}
/// Inputs needed to construct a conditional request; an update always has a read base.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Preparation {
	/// Encode the initial empty snapshot after a confirmed NotFound.
	Bootstrap,
	/// Merge the typed patch into this snapshot, preserving its ETag as the condition.
	Update { base: StoredConfig, patch: Vec<u8> },
}

/// Read errors distinguish transient transport faults from a malformed persisted value.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReadResult {
	/// Store supplied a complete object with a nonempty ETag.
	Found(StoredConfig),
	/// NotFound only; never synthesized from a failed request.
	Missing,
	/// Retry without inferring absence or write failure.
	Unknown(String),
	/// Invalid encoding, missing ETag or an oversized document; never overwrite it.
	Invalid(String),
}
/// Selects bootstrap, one-shot CLI work or the per-process Always service.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Purpose {
	/// May create an absent object; succeeds after applying a valid snapshot, without UPS.
	Bootstrap,
	/// Bounded read/apply operation; absence is an error.
	Refresh,
	/// Continuous refresh by periodic S3 reconciliation.
	Follow,
	/// JSON encoding of the existing DynamicConfigUpdate payload; merged once against a read base.
	Update { patch: Vec<u8> },
}
/// Runtime bounds, injectable in tests; no timer state is duplicated here.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Settings {
	/// Maximum duration of each executor exchange.
	pub io_timeout_ms: u64,
	/// Backoff after transient failure or unresolved readback.
	pub retry_delay_ms: u64,
	/// Periodic refresh repairs lost notifications, including commit-before-publish crashes.
	pub refresh_interval_ms: u64,
	/// Overall bound for bootstrap and CLI operations, including all retries.
	pub operation_timeout_ms: u64,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			io_timeout_ms: 5_000,
			retry_delay_ms: 1_000,
			refresh_interval_ms: 60_000,
			operation_timeout_ms: 30_000,
		}
	}
}
impl Settings {
	/// Reject zero intervals before spawning a runner that could spin on overdue alarms.
	pub fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.io_timeout_ms > 0
				&& self.retry_delay_ms > 0
				&& self.refresh_interval_ms > 0
				&& self.operation_timeout_ms > 0,
			"config state machine intervals must be positive"
		);
		Ok(())
	}
}
/// Runtime alarms; only the I/O handler decides when an executor response has timed out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Timer {
	/// Overall bootstrap/CLI bound, independent of an individual I/O timeout.
	Deadline,
	/// Periodic refresh or failed-read backoff, with one scheduled wake per process.
	Refresh,
	/// Ordinary read, prepare or apply timeout for this exact operation.
	Io(IoTimer<OpId>),
	/// Shared conditional write/readback/backoff progress.
	Publication(rivet_object_store_util::conditional_write::Timer<OpId>),
}
