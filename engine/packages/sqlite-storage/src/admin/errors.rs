use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("sqlite_admin")]
pub enum SqliteAdminError {
	#[error(
		"actor_restore_in_progress",
		"SQLite restore is in progress for this actor."
	)]
	ActorRestoreInProgress,
}
