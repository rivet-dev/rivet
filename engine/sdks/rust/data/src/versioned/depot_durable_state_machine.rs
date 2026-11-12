use crate::generated::depot_durable_state_machine_v1 as wire;

pub enum Data {
	V1(wire::Data),
}
impl_versioned_v1!(Data, DEPOT_DURABLE_STATE_MACHINE, wire::Data);

pub enum TimerEntry {
	V1(wire::TimerEntry),
}
impl_versioned_v1!(TimerEntry, DEPOT_DURABLE_STATE_MACHINE, wire::TimerEntry);
