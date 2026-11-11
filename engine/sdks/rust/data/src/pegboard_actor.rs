//! The latest `pegboard.actor` schema, which every actor record and timer is written in once the
//! fleet negotiates it.
pub use crate::generated::pegboard_actor_v6::*;
use vbare::Versioned;

/// The version a record is written at: the negotiated `pegboard.actor` version, capped at the
/// newest version the record envelopes know. The negotiated version is a ceiling, not a
/// requirement to emit a newer record.
pub const fn record_write_version(negotiated: u16) -> u16 {
	if negotiated < crate::versioned::ActorState::LATEST {
		negotiated
	} else {
		crate::versioned::ActorState::LATEST
	}
}
