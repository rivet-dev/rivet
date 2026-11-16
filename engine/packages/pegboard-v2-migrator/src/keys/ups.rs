//! UPS subjects owned by the V2 migrator.
use rivet_topology::worker::WorkerId;

// Operator requests to the migrator of the singleton tenure held by `worker_id`.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [RIVET, V2_MIGRATOR, worker_id])]
pub struct V2MigratorSubject {
	worker_id: WorkerId,
	#[subject]
	subject: Vec<u8>,
}
