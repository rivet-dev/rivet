//! UPS subjects owned by pegboard.
use uuid::Uuid;

rivet_keys::ups_subject! { pub EnvoySubject(connection_id: Uuid) => [RIVET, ENVOY, {connection_id}]; }

// Asks one superseded Envoy connection to close.
rivet_keys::ups_subject! { pub EnvoySupersedeSubject(connection_id: Uuid) => [RIVET, ENVOY, SUPERSEDE, {connection_id}]; }

// Start notices to one gateway wait. The waiting gateway subscribes per wait id.
rivet_keys::ups_subject! { pub ActorStartNoticeSubject(wait_id: u64) => [RIVET, ACTOR, START_NOTICE, {wait_id}]; }
