//! Exact worker-addressed subjects for replicated volume traffic.

use rivet_topology::worker::WorkerId;

rivet_keys::ups_subject! {
	pub VolumeLogSubject(worker_id: WorkerId) => [DEPOT, VOLUME_LOG, {worker_id}];
}

rivet_keys::ups_subject! {
	pub VolumeCommandSubject(worker_id: WorkerId) => [DEPOT, VOLUME_COMMAND, {worker_id}];
}

rivet_keys::ups_subject! {
	pub TabletCoordinatorSubject(worker_id: WorkerId) => [DEPOT, COORDINATOR, {worker_id}];
}

rivet_keys::ups_subject! {
	pub KvCommandSubject(worker_id: WorkerId) => [DEPOT, KV_COMMAND, {worker_id}];
}

rivet_keys::ups_subject! {
 pub StopReadinessSubject(worker_id: WorkerId) => [DEPOT, STOP_READINESS, {worker_id}];
}
