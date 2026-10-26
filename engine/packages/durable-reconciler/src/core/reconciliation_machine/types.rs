use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
	Work,
}

impl rivet_state_machine::Slot for Slot {}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Timer {
	Poll,
	/// Bounds exactly one effect attempt; an old alarm cannot cancel newer work.
	EffectTimeout(rivet_state_machine::OpId<Slot>),
}
