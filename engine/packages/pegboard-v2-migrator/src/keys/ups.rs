//! UPS subjects owned by the V2 migrator.
use rivet_topology::worker::WorkerId;

// Operator requests to the migrator of the singleton tenure held by `worker_id`.
rivet_keys::ups_subject! { pub V2MigratorSubject(worker_id: WorkerId) => [RIVET, V2_MIGRATOR, {worker_id}]; }
