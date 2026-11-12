//! UPS subjects owned by pegboard.
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [RIVET, ENVOY, connection_id])]
pub struct EnvoySubject {
	connection_id: Uuid,
	#[subject]
	subject: Vec<u8>,
}

// Asks one superseded Envoy connection to close.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [RIVET, ENVOY, SUPERSEDE, connection_id])]
pub struct EnvoySupersedeSubject {
	connection_id: Uuid,
	#[subject]
	subject: Vec<u8>,
}

// Start notices to one gateway wait. The waiting gateway subscribes per wait id.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [RIVET, ACTOR, START_NOTICE, wait_id])]
pub struct ActorStartNoticeSubject {
	wait_id: u64,
	#[subject]
	subject: Vec<u8>,
}

// Start notices to one create waiting for a namespace's fork. The waiting route subscribes per
// wait id.
#[derive(Clone, Debug, PartialEq, Eq, rivet_keys::UpsSubject)]
#[ups(subject = [RIVET, NAMESPACE, START_NOTICE, wait_id])]
pub struct NamespaceStartNoticeSubject {
	wait_id: u64,
	#[subject]
	subject: Vec<u8>,
}
