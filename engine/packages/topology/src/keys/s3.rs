use crate::worker::{WorkerId, WorkerRole};

// Storage nodes must be listable for placement. Stateless workers have too high a
// cardinality for discovery listing; only bounded background GC may list them.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, DEPOT, CLUSTER, NODE, STORAGE], value = unit)]
pub struct StorageHeartbeatPrefix {}

// Role-specific listing for background GC, never for stateless discovery or routing.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, DEPOT, CLUSTER, NODE, role], value = unit)]
pub struct WorkerHeartbeatPrefix {
	pub role: WorkerRole,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::S3Key)]
#[s3(key = [RIVET, DEPOT, CLUSTER, NODE, role, worker_id], value = vbare_selected(rivet_data::versioned::TopologyWorkerHeartbeatData))]
pub struct WorkerHeartbeatKey {
	pub role: WorkerRole,
	pub worker_id: WorkerId,
}
