//! Exclusive control lane. A publication retains its exact bytes and original ETag.
use super::types::*;
use rivet_object_store_util::conditional_write::ConditionalWrite;
use rivet_state_machine::OpSequence;
use rivet_state_machine_util::IoCall;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
	/// One read is in flight; no write has been prepared.
	Reading(IoCall<OpId>),
	/// Entropy is requested only for bootstrap or staging a new pending key.
	Generating {
		call: IoCall<OpId>,
		base: Option<StoredRing>,
	},
	/// This helper owns all retry and readback state for an immutable mutation.
	Publishing(ConditionalWrite<Mutation, OpId>),
	/// No I/O is outstanding. Wake is the next lifecycle deadline.
	Waiting {
		stored: StoredRing,
		wake_unix_ms: i64,
	},
	/// A read will run after the retry delay, without overwriting unreadable data.
	Backoff,
	/// Authority ended or a newer epoch was observed. No further effects are allowed.
	Stopped,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
	pub settings: Settings,
	pub phase: Phase,
	pub sequence: OpSequence<Slot>,
	pub diagnostic: Option<String>,
	/// Retains confirmed generation knowledge across retries, so deletion cannot reset the ring.
	pub generation_floor: u64,
	pub emergency: Option<EmergencyCall>,
}
impl State {
	pub fn new(settings: Settings) -> anyhow::Result<Self> {
		settings.policy.validate()?;
		anyhow::ensure!(settings.epoch > 0, "JWT coordinator epoch must be positive");
		Ok(Self {
			settings,
			phase: Phase::Backoff,
			sequence: OpSequence::default(),
			diagnostic: None,
			generation_floor: 0,
			emergency: None,
		})
	}
	pub fn pending(&self) -> Option<&IoCall<OpId>> {
		match &self.phase {
			Phase::Reading(call) | Phase::Generating { call, .. } => Some(call),
			Phase::Publishing(progress) => progress.pending(),
			Phase::Waiting { .. } | Phase::Backoff | Phase::Stopped => None,
		}
	}
}
impl rivet_state_machine::StateMachineState for State {}
