//! KV rows of a group volume and of an event shard; see `depot.group.v1.bare` for their meaning.

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [GROUP_RECORD], value = vbare(rivet_data::versioned::DepotGroupRecordData, rivet_data::DEPOT_GROUP_VERSION))]
pub struct GroupRecordKey {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [FORK_CHILD, fork_number], value = vbare(rivet_data::versioned::DepotForkChildData, rivet_data::DEPOT_GROUP_VERSION))]
pub struct ForkChildKey {
	pub fork_number: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [FORK_CHILD])]
pub struct ForkChildPrefix {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [FORK_OPERATION, operation_id], value = vbare(rivet_data::versioned::DepotForkOperationData, rivet_data::DEPOT_GROUP_VERSION))]
pub struct ForkOperationKey {
	pub operation_id: uuid::Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [EVENT_SEQ], value = vbare(rivet_data::versioned::DepotEventCounterData, rivet_data::DEPOT_GROUP_VERSION))]
pub struct EventSeqKey {}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::VolumeKvKey)]
#[kv(key = [EVENT_ROW, sequence], value = vbare(rivet_data::versioned::DepotEventData, rivet_data::DEPOT_GROUP_VERSION))]
pub struct EventRowKey {
	pub sequence: u64,
}

impl ForkChildKey {
	pub fn fork_number(&self) -> u64 {
		self.fork_number
	}
}

impl EventRowKey {
	pub fn sequence(&self) -> u64 {
		self.sequence
	}
}
