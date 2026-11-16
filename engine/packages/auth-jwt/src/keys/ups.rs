//! Emergency rotation requests address the existing topology worker holding the singleton.
use rivet_topology::worker::WorkerId;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [RIVET, JWT_ROTATION, worker_id])]
pub struct JwtRotationSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}
