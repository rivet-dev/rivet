//! The values the runner reports as signal events, read from state in constant time.

use super::MaintainerState;
use crate::tablet_maintainer_machine::SignalSnapshot;

pub(crate) fn snapshot(state: &MaintainerState) -> SignalSnapshot {
	let index = state.operations.index();
	let learner = super::catch_up::learner(state);
	SignalSnapshot {
		above_cold_bytes: state
			.filesystem
			.unflushed_total()
			.saturating_add(state.filesystem.carried_bytes())
			.saturating_add(index.apply_bytes),
		maintainer_log_replaying: super::maintainer_log::replay::replaying(state),
		apply_idle: index.apply_bytes == 0
			&& index.counts.filesystem_read == 0
			&& state.filesystem.carried_bytes() == 0
			&& state.store_queue.len() == 0,
		learner: learner.map(|worker| *worker.as_bytes()),
		uncovered_volumes: state.membership_checks.uncovered_count() as u32,
	}
}
