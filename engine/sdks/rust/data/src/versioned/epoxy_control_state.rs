use crate::generated::epoxy_control_state_v1;

pub enum EpoxyCoordinatorStateData {
	V1(epoxy_control_state_v1::CoordinatorData),
}

impl_versioned_v1!(
	EpoxyCoordinatorStateData,
	epoxy_control_state_v1::CoordinatorData
);

pub enum EpoxyReplicaStateData {
	V1(epoxy_control_state_v1::ReplicaData),
}

impl_versioned_v1!(EpoxyReplicaStateData, epoxy_control_state_v1::ReplicaData);
