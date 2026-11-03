//! Read-only view of the migrator for the service that runs it.
use super::{state::*, types::*};

/// Every phase an in-flight entry can be in, in migration order. The per-phase gauge reports each
/// one, zero included, so a phase that empties drops to zero instead of keeping its last value.
pub const PHASES: [&str; 6] = [
	"copying",
	"reserving",
	"cutting_over",
	"confirming",
	"activating",
	"discarding",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
	/// A newer tenure fenced this one, or it was asked to stop.
	pub stopped: bool,
	/// In-flight entries per phase, indexed like [`PHASES`].
	pub in_flight_by_phase: [usize; PHASES.len()],
	/// When the oldest in-flight entry was admitted, in unix ms. An entry retrying a step it cannot
	/// finish keeps this old.
	pub oldest_started_at: Option<i64>,
}

pub fn phase_name(phase: &wire::MigrateFromV2Phase) -> &'static str {
	match phase {
		wire::MigrateFromV2Phase::MigrateFromV2Copying => "copying",
		wire::MigrateFromV2Phase::MigrateFromV2Reserving => "reserving",
		wire::MigrateFromV2Phase::MigrateFromV2CuttingOver(_) => "cutting_over",
		wire::MigrateFromV2Phase::MigrateFromV2Confirming(_) => "confirming",
		wire::MigrateFromV2Phase::MigrateFromV2Activating(_) => "activating",
		wire::MigrateFromV2Phase::MigrateFromV2Discarding(_) => "discarding",
	}
}

pub fn build(state: &State) -> Snapshot {
	let mut in_flight_by_phase = [0; PHASES.len()];
	for entry in state.entries.values() {
		let name = phase_name(&entry.record.phase);
		if let Some(index) = PHASES.iter().position(|phase| *phase == name) {
			in_flight_by_phase[index] += 1;
		}
	}
	Snapshot {
		stopped: matches!(state.phase, Phase::Stopped),
		in_flight_by_phase,
		oldest_started_at: state
			.entries
			.values()
			.map(|entry| entry.record.started_at)
			.min(),
	}
}
