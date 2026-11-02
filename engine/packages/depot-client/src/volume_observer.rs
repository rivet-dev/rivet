//! Observes each volume client a process runs, on the runner task: its log bytes above cold
//! for the process metric, and its first terminal phase for the owner that releases its slot.

use crate::metrics;
use depot_volume::volume_client_machine::{
	snapshot::VolumeClientSnapshot, state::VolumeClientPhase,
};
use parking_lot::Mutex;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use rivet_state_machine_ephemeral::SnapshotObserver;
use std::sync::atomic::{AtomicBool, Ordering};

/// The fields of a client snapshot the observer reads, copied on the runner task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sample {
	pub dirty_bytes: u64,
	/// The runner stopped or was fenced; its owner can release the slot.
	pub terminal: bool,
}

impl Sample {
	fn from_snapshot(snapshot: &VolumeClientSnapshot) -> Self {
		Self {
			dirty_bytes: snapshot.dirty_bytes,
			terminal: matches!(
				snapshot.phase,
				VolumeClientPhase::Stopped | VolumeClientPhase::Fenced { .. }
			),
		}
	}
}

/// Owned by the runner. Its share of `depot_client_dirty_bytes` ends with it.
pub(crate) struct VolumeObserver {
	/// This runner's share of the process's log bytes above cold.
	dirty: Mutex<Option<IntGaugeGuard>>,
	/// Runs once when the runner first reaches a terminal phase.
	on_terminal: Option<Box<dyn Fn() + Send + Sync>>,
	terminal_signalled: AtomicBool,
}

impl VolumeObserver {
	pub(crate) fn new(on_terminal: Option<Box<dyn Fn() + Send + Sync>>) -> Self {
		Self {
			dirty: Mutex::new(None),
			on_terminal,
			terminal_signalled: AtomicBool::new(false),
		}
	}
}

impl SnapshotObserver<VolumeClientSnapshot> for VolumeObserver {
	type Sample = Sample;

	fn sample(&self, snapshot: &VolumeClientSnapshot) -> Sample {
		Sample::from_snapshot(snapshot)
	}

	fn deliver(&self, sample: Sample) {
		if sample.terminal
			&& !self.terminal_signalled.swap(true, Ordering::AcqRel)
			&& let Some(on_terminal) = &self.on_terminal
		{
			on_terminal();
		}
		// Forced-sync: the observer runs on the runner task and never awaits.
		let mut dirty = self.dirty.lock();
		*dirty = None;
		*dirty = Some(metrics::DIRTY_BYTES.add_guard(sample.dirty_bytes as i64));
	}
}
