//! Rows the durable runner keeps in each machine's volume. Only the runner writes these rows.

use bytes::Bytes;
use rivet_keys::keys::OrderedI64;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_STATE], value = bytes)]
pub struct MachineStateKey {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_DATA], value = vbare(rivet_data::versioned::Data, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION))]
pub struct MachineDataKey {}

// An armed timer, ordered by deadline and named by the machine's stable timer id. The value is
// the timer as the machine serializes it, so the payload's encoding can change without changing
// which row is the timer.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_TIMER, deadline_unix_ms, timer_id], value = bytes)]
pub struct MachineTimerKey {
	pub deadline_unix_ms: OrderedI64,
	pub timer_id: Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_TIMER])]
pub struct MachineTimerPrefix {}

// The deadline of the armed timer with this id, so arming and cancelling find its timer row.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_TIMER_BY_ID, timer_id], value = vbare(rivet_data::versioned::TimerEntry, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION))]
pub struct MachineTimerByIdKey {
	pub timer_id: Bytes,
}

// Timers armed before timers had stable ids, named by their serialized bytes. The runner reads
// them on every pass and moves each into the rows above in its next commit.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_TIMER_LEGACY, deadline_unix_ms, timer], value = unit)]
pub struct LegacyMachineTimerKey {
	pub deadline_unix_ms: OrderedI64,
	pub timer: Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_TIMER_LEGACY])]
pub struct LegacyMachineTimerPrefix {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [MACHINE_TIMER_BY_ID_LEGACY, timer], value = vbare(rivet_data::versioned::TimerEntry, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION))]
pub struct LegacyMachineTimerByIdKey {
	pub timer: Bytes,
}
