//! Exact worker-addressed subjects for replicated volume traffic.

use rivet_topology::worker::WorkerId;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [DEPOT, VOLUME_LOG, worker_id])]
pub struct VolumeLogSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [DEPOT, VOLUME_COMMAND, worker_id])]
pub struct VolumeCommandSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [DEPOT, COORDINATOR, worker_id])]
pub struct TabletCoordinatorSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [DEPOT, KV_COMMAND, worker_id])]
pub struct KvCommandSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [DEPOT, STOP_READINESS, worker_id])]
pub struct StopReadinessSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [DEPOT, REPLICA_READINESS, worker_id])]
pub struct ReplicaReadinessSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}
