//! Rows the durable runner keeps in each machine's volume. Only the runner writes these rows.

use bytes::Bytes;
use rivet_keys::keys::OrderedI64;

rivet_keys::volume_kv_key! {
	pub MachineStateKey() => [MACHINE_STATE];
	value: bytes;
}

rivet_keys::volume_kv_key! {
	pub MachineDataKey() => [MACHINE_DATA];
	value: vbare(rivet_data::versioned::Data, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION);
}

// An armed timer, ordered by deadline and named by the machine's stable timer id. The value is
// the timer as the machine serializes it, so the payload's encoding can change without changing
// which row is the timer.
rivet_keys::volume_kv_key! {
	pub MachineTimerKey(deadline_unix_ms: OrderedI64, timer_id: Bytes) => [MACHINE_TIMER, {deadline_unix_ms}, {timer_id}];
	value: bytes;
}

rivet_keys::volume_kv_key! {
	pub MachineTimerPrefix() => [MACHINE_TIMER];
}

// The deadline of the armed timer with this id, so arming and cancelling find its timer row.
rivet_keys::volume_kv_key! {
	pub MachineTimerByIdKey(timer_id: Bytes) => [MACHINE_TIMER_BY_ID, {timer_id}];
	value: vbare(rivet_data::versioned::TimerEntry, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION);
}

// Timers armed before timers had stable ids, named by their serialized bytes. The runner reads
// them on every pass and moves each into the rows above in its next commit.
rivet_keys::volume_kv_key! {
	pub LegacyMachineTimerKey(deadline_unix_ms: OrderedI64, timer: Bytes) => [MACHINE_TIMER_LEGACY, {deadline_unix_ms}, {timer}];
	value: unit;
}

rivet_keys::volume_kv_key! {
	pub LegacyMachineTimerPrefix() => [MACHINE_TIMER_LEGACY];
}

rivet_keys::volume_kv_key! {
	pub LegacyMachineTimerByIdKey(timer: Bytes) => [MACHINE_TIMER_BY_ID_LEGACY, {timer}];
	value: vbare(rivet_data::versioned::TimerEntry, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION);
}
