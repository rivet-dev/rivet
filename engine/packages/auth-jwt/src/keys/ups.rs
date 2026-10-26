//! Emergency rotation requests address the existing topology worker holding the singleton.
use rivet_topology::worker::WorkerId;

rivet_keys::ups_subject! {
	pub JwtRotationSubject(worker_id: WorkerId) => [RIVET, JWT_ROTATION, {worker_id}];
}
