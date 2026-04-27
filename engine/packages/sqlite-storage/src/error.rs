use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SqliteStorageError {
	#[error("sqlite meta missing for {operation}")]
	MetaMissing { operation: &'static str },

	#[error("sqlite db is not open for {operation}")]
	DbNotOpen { operation: &'static str },

	#[error("FenceMismatch: {reason}")]
	FenceMismatch { reason: String },

	#[error(
		"CommitTooLarge: raw dirty pages were {actual_size_bytes} bytes, limit is {max_size_bytes} bytes"
	)]
	CommitTooLarge {
		actual_size_bytes: u64,
		max_size_bytes: u64,
	},

	#[error("StageNotFound: stage {stage_id} missing")]
	StageNotFound { stage_id: u64 },

	#[error("invalid sqlite v1 migration state")]
	InvalidV1MigrationState,
}
