use rivet_error::RivetError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(RivetError, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[error("sqlite_admin")]
pub enum SqliteAdminError {
	#[error(
		"invalid_restore_point",
		"the requested target is not within the retention window or has had its DELTAs cleaned up"
	)]
	InvalidRestorePoint {
		target_txid: u64,
		reachable_hints: Vec<u64>,
	},

	#[error(
		"fork_destination_exists",
		"the destination actor already has SQLite state"
	)]
	ForkDestinationAlreadyExists { dst_actor_id: String },

	#[error(
		"pitr_disabled_for_namespace",
		"PITR is not enabled for this namespace"
	)]
	PitrDisabledForNamespace,

	#[error(
		"pitr_destructive_disabled_for_namespace",
		"destructive PITR (Apply mode restore) is not enabled for this namespace"
	)]
	PitrDestructiveDisabledForNamespace,

	#[error(
		"pitr_admin_disabled_for_namespace",
		"PITR admin operations are not enabled for this namespace"
	)]
	PitrAdminDisabledForNamespace,

	#[error(
		"fork_disabled_for_namespace",
		"SQLite fork operations are not enabled for this namespace"
	)]
	ForkDisabledForNamespace,

	#[error(
		"retention_window_exceeded",
		"target predates the retention window"
	)]
	RetentionWindowExceeded { oldest_reachable_txid: u64 },

	#[error(
		"restore_in_progress",
		"a restore operation is already running on this actor"
	)]
	RestoreInProgress { existing_operation_id: Uuid },

	#[error(
		"fork_in_progress",
		"a fork operation is already targeting this destination actor"
	)]
	ForkInProgress { existing_operation_id: Uuid },

	#[error(
		"actor_restore_in_progress",
		"the actor is being restored; commits are temporarily blocked"
	)]
	ActorRestoreInProgress,

	#[error(
		"admin_op_rate_limited",
		"too many concurrent admin operations for this namespace"
	)]
	AdminOpRateLimited { retry_after_ms: u64 },

	#[error(
		"pitr_namespace_budget_exceeded",
		"creating this checkpoint would exceed the namespace PITR budget"
	)]
	PitrNamespaceBudgetExceeded {
		used_bytes: u64,
		budget_bytes: u64,
	},

	#[error(
		"operation_orphaned",
		"operation has been pending without a working pod for too long; please retry"
	)]
	OperationOrphaned { operation_id: Uuid },
}
