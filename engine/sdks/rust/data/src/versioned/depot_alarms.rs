//! Volume alarm values and the actor, manifest and coordinator envelopes that carry alarm state.

pub enum DepotVolumeAlarmData {
	V1(crate::generated::depot_alarm_v1::AlarmValue),
}
impl_versioned_v1!(
	DepotVolumeAlarmData,
	crate::generated::depot_alarm_v1::AlarmValue
);

pub enum DepotAlarmCurrentData {
	V1(crate::generated::depot_alarm_v1::AlarmCurrent),
}
impl_versioned_v1!(
	DepotAlarmCurrentData,
	crate::generated::depot_alarm_v1::AlarmCurrent
);

pub enum DepotAlarmTimeData {
	V1(crate::generated::depot_alarm_v1::AlarmTimeRecord),
}
impl_versioned_v1!(
	DepotAlarmTimeData,
	crate::generated::depot_alarm_v1::AlarmTimeRecord
);

pub enum ActorAlarmRequestData {
	V1(crate::generated::pegboard_actor_alarm_v1::ActorAlarmRequest),
}
impl_versioned_v1!(
	ActorAlarmRequestData,
	crate::generated::pegboard_actor_alarm_v1::ActorAlarmRequest
);

pub enum ActorSetAlarmRequestData {
	V1(crate::generated::pegboard_actor_alarm_v1::ActorSetAlarmRequest),
}
impl_versioned_v1!(
	ActorSetAlarmRequestData,
	crate::generated::pegboard_actor_alarm_v1::ActorSetAlarmRequest
);

pub enum DepotAlarmIndexProgressData {
	V1(crate::generated::depot_tablet_manifest_v1::AlarmIndexProgress),
}
impl_versioned_v1!(
	DepotAlarmIndexProgressData,
	crate::generated::depot_tablet_manifest_v1::AlarmIndexProgress
);

/// The actor state machine's persisted state, with its alarm schedule.
pub enum ActorAlarmMachineStateData {
	V1(crate::generated::pegboard_actor_state_v1::ActorMachineState),
}
impl_versioned_v1!(
	ActorAlarmMachineStateData,
	crate::generated::pegboard_actor_state_v1::ActorMachineState
);

impl std::ops::Deref for crate::generated::depot_tablet_manifest_v1::AlarmTabletManifest {
	type Target = crate::generated::depot_tablet_manifest_v1::TabletManifest;
	fn deref(&self) -> &Self::Target {
		&self.manifest
	}
}

/// The stored tablet manifest object, with each tablet's alarm index initialization.
pub enum DepotAlarmTabletManifestData {
	V1(crate::generated::depot_tablet_manifest_v1::AlarmTabletManifest),
}
impl_versioned_v1!(
	DepotAlarmTabletManifestData,
	crate::generated::depot_tablet_manifest_v1::AlarmTabletManifest
);

/// Every request a tablet coordinator receives.
pub enum DepotAlarmCoordinatorRequestData {
	V1(crate::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest),
}
impl_versioned_v1!(
	DepotAlarmCoordinatorRequestData,
	crate::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest
);
