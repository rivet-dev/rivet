//! Versioned rows of group volumes and event shards.

/// Versioned `GroupRecord` with strict payload decoding.
pub enum DepotGroupRecordData {
	V1(crate::generated::depot_group_v1::GroupRecord),
}

impl_versioned_v1!(
	DepotGroupRecordData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::GroupRecord
);

/// Versioned `ForkChild` with strict payload decoding.
pub enum DepotForkChildData {
	V1(crate::generated::depot_group_v1::ForkChild),
}

impl_versioned_v1!(
	DepotForkChildData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::ForkChild
);

/// Versioned `EventCounter` with strict payload decoding.
pub enum DepotEventCounterData {
	V1(crate::generated::depot_group_v1::EventCounter),
}

impl_versioned_v1!(
	DepotEventCounterData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::EventCounter
);

/// Versioned `Event` with strict payload decoding.
pub enum DepotEventData {
	V1(crate::generated::depot_group_v1::Event),
}

impl_versioned_v1!(
	DepotEventData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::Event
);

/// Versioned `GroupReadRequest` with strict payload decoding.
pub enum DepotGroupReadRequestData {
	V1(crate::generated::depot_group_v1::GroupReadRequest),
}

impl_versioned_v1!(
	DepotGroupReadRequestData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::GroupReadRequest
);

/// Versioned `GroupReadResponse` with strict payload decoding.
pub enum DepotGroupReadResponseData {
	V1(crate::generated::depot_group_v1::GroupReadResponse),
}

impl_versioned_v1!(
	DepotGroupReadResponseData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::GroupReadResponse
);

/// Versioned `EventsReadRequest` with strict payload decoding.
pub enum DepotEventsReadRequestData {
	V1(crate::generated::depot_group_v1::EventsReadRequest),
}

impl_versioned_v1!(
	DepotEventsReadRequestData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::EventsReadRequest
);

/// Versioned `EventsReadResponse` with strict payload decoding.
pub enum DepotEventsReadResponseData {
	V1(crate::generated::depot_group_v1::EventsReadResponse),
}

impl_versioned_v1!(
	DepotEventsReadResponseData,
	DEPOT_GROUP,
	crate::generated::depot_group_v1::EventsReadResponse
);

macro_rules! group_envelope {
	($name:ident, $ty:ident) => {
		#[doc = concat!("Versioned `", stringify!($ty), "` with strict payload decoding.")]
		pub enum $name {
			V1(crate::generated::depot_group_v1::$ty),
		}

		impl_versioned_v1!($name, DEPOT_GROUP, crate::generated::depot_group_v1::$ty);
	};
}

group_envelope!(DepotForkOperationData, ForkOperation);
group_envelope!(DepotGroupCreateRequestData, GroupCreateRequest);
group_envelope!(DepotGroupCreateResponseData, GroupCreateResponse);
group_envelope!(DepotGroupForkDrawRequestData, GroupForkDrawRequest);
group_envelope!(DepotGroupForkDrawResponseData, GroupForkDrawResponse);
group_envelope!(DepotGroupChildCreateRequestData, GroupChildCreateRequest);
group_envelope!(DepotGroupChildCreateResponseData, GroupChildCreateResponse);
group_envelope!(DepotGroupArchiveRequestData, GroupArchiveRequest);
group_envelope!(DepotGroupArchiveResponseData, GroupArchiveResponse);
group_envelope!(
	DepotGroupChildArchivedRequestData,
	GroupChildArchivedRequest
);
group_envelope!(
	DepotGroupChildArchivedResponseData,
	GroupChildArchivedResponse
);
group_envelope!(DepotEventPostRequestData, EventPostRequest);
group_envelope!(DepotEventPostResponseData, EventPostResponse);
