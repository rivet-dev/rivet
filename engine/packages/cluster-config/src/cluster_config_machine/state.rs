//! Serializable transition state; Config handles, storage clients and tasks belong to the executor.
use super::types::*;
use rivet_object_store_util::conditional_write::{ConditionalWrite, ControlIo};
use rivet_state_machine::OpSequence;
use serde::{Deserialize, Serialize};

/// Exclusive activity; publication itself contains the shared write/readback/wait substates.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
	/// Constructed without I/O, or following an applied snapshot until refresh is requested.
	Idle,
	/// A read of the authoritative object is outstanding.
	Reading { call: IoCall<OpId> },
	/// Merge/validation is pending; the executor returns the exact prepared request.
	Preparing { call: IoCall<OpId> },
	/// Immutable request survives unknown PUTs, readbacks and alarm-driven retries.
	Publishing(ConditionalWrite<PreparedConfig, OpId>),
	/// The executor applies this accepted snapshot through the existing Config interface.
	Applying { call: IoCall<OpId>, etag: String },
	/// A read/apply failure waits for the refresh alarm; last valid settings remain active.
	Waiting,
	/// No further effects are permitted; a possibly sent PUT remains diagnostic evidence.
	Finished {
		/// Success means the requested snapshot was applied locally after storage confirmation.
		result: Result<(), String>,
		/// Cancellation/timeouts cannot retract a sent conditional write.
		unresolved: Option<PreparedConfig>,
	},
}
impl Phase {
	/// Executor token and deadline derived from current activity, never independently tracked.
	pub fn pending(&self) -> Option<&IoCall<OpId>> {
		self.io().pending()
	}

	/// Maps domain activities without duplicating their operation tokens or publication state.
	fn io(&self) -> ControlIo<'_, PreparedConfig, OpId> {
		match self {
			Self::Reading { call } => ControlIo::Reading(call),
			Self::Preparing { call } | Self::Applying { call, .. } => ControlIo::Other(call),
			Self::Publishing(progress) => ControlIo::Publishing(progress),
			Self::Idle | Self::Waiting | Self::Finished { .. } => ControlIo::Idle,
		}
	}
}
/// One bounded operation or a continuous per-process config follower; never a singleton lease holder.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
	/// Immutable purpose selected before startup.
	pub purpose: Purpose,
	/// Positive operational bounds checked at construction.
	pub settings: Settings,
	/// Sole authority for current executor work.
	pub phase: Phase,
	/// Allocator survives replacement of publication progress.
	pub sequence: OpSequence<Slot>,
	/// Set at startup for bounded work; Follow has no overall deadline.
	pub operation_deadline_boot_ms: Option<u64>,
	/// Only updated after a successful ApplyConfigResponse.
	pub applied_etag: Option<String>,
	/// One coalesced notification arriving during work requires another read after that work.
	pub refresh_requested: bool,
	/// Most recent failure for diagnostics; never replaces the last valid process configuration.
	pub diagnostic: Option<String>,
}
impl State {
	/// Construction is pure; Runner startup emits the initial read.
	pub fn new(purpose: Purpose, settings: Settings) -> anyhow::Result<Self> {
		settings.validate()?;
		Ok(Self {
			purpose,
			settings,
			phase: Phase::Idle,
			sequence: OpSequence::default(),
			operation_deadline_boot_ms: None,
			applied_etag: None,
			refresh_requested: false,
			diagnostic: None,
		})
	}
}
