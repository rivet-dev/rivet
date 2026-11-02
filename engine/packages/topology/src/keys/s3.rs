use crate::worker::{WorkerId, WorkerRole};

// Storage nodes must be listable for placement. Stateless workers have too high a
// cardinality for discovery listing; only bounded background GC may list them.
rivet_keys::s3_key! {
	pub StorageHeartbeatPrefix() => [RIVET, DEPOT, CLUSTER, NODE, STORAGE];
	value: unit;
}

// Role-specific listing for background GC, never for stateless discovery or routing.
rivet_keys::s3_key! {
	pub WorkerHeartbeatPrefix(pub role: WorkerRole) => [RIVET, DEPOT, CLUSTER, NODE, {role}];
	value: unit;
}

rivet_keys::s3_key! {
	pub WorkerHeartbeatKey(pub role: WorkerRole, pub worker_id: WorkerId) => [RIVET, DEPOT, CLUSTER, NODE, {role}, {worker_id}];
	value: vbare(rivet_data::versioned::TopologyWorkerHeartbeatData, rivet_data::TOPOLOGY_WORKER_HEARTBEAT_VERSION);
}
