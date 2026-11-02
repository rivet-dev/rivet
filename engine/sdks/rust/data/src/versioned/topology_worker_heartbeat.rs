use crate::generated::topology_worker_heartbeat_v1;

/// Every worker, stateless ones included, reads heartbeats directly, and
/// `Topology::version_for` counts only storage workers. After release, a new version ships in
/// two deploys: the first binary reads it but still writes the previous version, and only the
/// second writes it. Skipping the first deploy is unsupported.
pub enum TopologyWorkerHeartbeatData {
	V1(topology_worker_heartbeat_v1::Data),
}

impl_versioned_v1!(
	TopologyWorkerHeartbeatData,
	topology_worker_heartbeat_v1::Data
);
