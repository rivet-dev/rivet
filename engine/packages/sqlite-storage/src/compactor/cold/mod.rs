pub mod lease;
pub mod phase_a;
pub mod phase_b;
pub mod worker;

pub use lease::{
	ColdCompactorLease, ColdRenewOutcome, ColdTakeOutcome, SQLITE_COLD_COMPACTOR_LEASE_VERSION,
	decode_cold_lease, encode_cold_lease, release, renew, take,
};
pub use phase_a::{
	ColdCompactState, ColdPendingMarker, ColdPhaseAPlan, SQLITE_COLD_COMPACT_STATE_VERSION,
	SQLITE_COLD_PENDING_MARKER_VERSION, decode_cold_compact_state, decode_pending_marker,
	encode_cold_compact_state, encode_pending_marker,
};
pub use phase_b::ColdPhaseBOutput;
pub use worker::{ColdCompactorConfig, start};
