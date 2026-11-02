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

rivet_keys::volume_kv_key! {
	pub MachineTimerKey(deadline_unix_ms: OrderedI64, timer: Bytes) => [MACHINE_TIMER, {deadline_unix_ms}, {timer}];
	value: unit;
}

rivet_keys::volume_kv_key! {
	pub MachineTimerPrefix() => [MACHINE_TIMER];
}

rivet_keys::volume_kv_key! {
	pub MachineTimerByIdKey(timer: Bytes) => [MACHINE_TIMER_BY_ID, {timer}];
	value: vbare(rivet_data::versioned::TimerEntry, rivet_data::generated::DEPOT_DURABLE_STATE_MACHINE_VERSION);
}
