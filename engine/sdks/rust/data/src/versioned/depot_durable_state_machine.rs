use crate::generated::depot_durable_state_machine_v1 as wire;

pub enum Data {
	V1(wire::Data),
}
impl_versioned_v1!(Data, wire::Data);

pub enum TimerEntry {
	V1(wire::TimerEntry),
}
impl_versioned_v1!(TimerEntry, wire::TimerEntry);
