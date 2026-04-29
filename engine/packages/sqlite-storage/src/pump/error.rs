use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SqliteStorageError {
	#[error("sqlite meta missing for {operation}")]
	MetaMissing { operation: &'static str },

	#[cfg(debug_assertions)]
	#[error("FenceMismatch: {reason}")]
	FenceMismatch { reason: String },

	#[error(
		"CommitTooLarge: raw dirty pages were {actual_size_bytes} bytes, limit is {max_size_bytes} bytes"
	)]
	CommitTooLarge {
		actual_size_bytes: u64,
		max_size_bytes: u64,
	},

	#[error("invalid sqlite v1 migration state")]
	InvalidV1MigrationState,
}
