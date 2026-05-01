use rivet_error::RivetError;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, RivetError)]
#[error("depot")]
pub enum SqliteStorageError {
	#[error(
		"meta_missing",
		"SQLite metadata is missing.",
		"SQLite metadata is missing for {operation}."
	)]
	MetaMissing { operation: &'static str },

	#[cfg(debug_assertions)]
	#[error(
		"fence_mismatch",
		"Depot debug fence mismatch.",
		"Depot debug fence mismatch: {reason}."
	)]
	FenceMismatch { reason: String },

	#[error(
		"commit_too_large",
		"SQLite commit payload is too large.",
		"SQLite commit payload is too large. Raw dirty pages were {actual_size_bytes} bytes, limit is {max_size_bytes} bytes."
	)]
	CommitTooLarge {
		actual_size_bytes: u64,
		max_size_bytes: u64,
	},

	#[error(
		"quota_exceeded",
		"Not enough space left in Depot.",
		"Not enough space left in Depot ({remaining_bytes} bytes remaining, current payload is {payload_size} bytes)."
	)]
	SqliteStorageQuotaExceeded {
		remaining_bytes: i64,
		payload_size: i64,
	},

	#[error("invalid_v1_migration_state", "Invalid SQLite v1 migration state.")]
	InvalidV1MigrationState,

	#[error(
		"fork_chain_too_deep",
		"Database branch fork chain is too deep."
	)]
	ForkChainTooDeep,

	#[error(
		"namespace_fork_chain_too_deep",
		"Namespace branch fork chain is too deep."
	)]
	NamespaceForkChainTooDeep,

	#[error(
		"fork_out_of_retention",
		"Cannot fork from a point that has fallen out of retention."
	)]
	ForkOutOfRetention,

	#[error(
		"bookmark_expired",
		"Bookmark history is no longer retained."
	)]
	BookmarkExpired,

	#[error(
		"branch_not_reachable",
		"Bookmark branch is not reachable from this database branch chain."
	)]
	BranchNotReachable,

	#[error(
		"shard_version_cap_exhausted",
		"SQLite shard version cap is exhausted."
	)]
	ShardVersionCapExhausted,

	#[error(
		"too_many_pins",
		"Namespace has too many pinned bookmarks."
	)]
	TooManyPins,

	#[error("database_not_found", "Database was not found in this namespace branch.")]
	DatabaseNotFound,
}

impl fmt::Display for SqliteStorageError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SqliteStorageError::MetaMissing { operation } => {
				write!(f, "sqlite meta missing for {operation}")
			}
			#[cfg(debug_assertions)]
			SqliteStorageError::FenceMismatch { reason } => {
				write!(f, "FenceMismatch: {reason}")
			}
			SqliteStorageError::CommitTooLarge {
				actual_size_bytes,
				max_size_bytes,
			} => write!(
				f,
				"CommitTooLarge: raw dirty pages were {actual_size_bytes} bytes, limit is {max_size_bytes} bytes"
			),
			SqliteStorageError::SqliteStorageQuotaExceeded {
				remaining_bytes,
				payload_size,
			} => write!(
				f,
				"SqliteStorageQuotaExceeded: not enough space left in depot ({remaining_bytes} bytes remaining, current payload is {payload_size} bytes)"
			),
			SqliteStorageError::InvalidV1MigrationState => {
				write!(f, "invalid sqlite v1 migration state")
			}
			SqliteStorageError::ForkChainTooDeep => {
				write!(f, "sqlite database branch fork chain is too deep")
			}
			SqliteStorageError::NamespaceForkChainTooDeep => {
				write!(f, "sqlite namespace branch fork chain is too deep")
			}
			SqliteStorageError::ForkOutOfRetention => {
				write!(f, "cannot fork from a point that has fallen out of retention")
			}
			SqliteStorageError::BookmarkExpired => {
				write!(f, "sqlite bookmark history is no longer retained")
			}
			SqliteStorageError::BranchNotReachable => {
				write!(f, "sqlite bookmark branch is not reachable from this database branch chain")
			}
			SqliteStorageError::ShardVersionCapExhausted => {
				write!(f, "sqlite shard version cap is exhausted")
			}
			SqliteStorageError::TooManyPins => {
				write!(f, "sqlite namespace has too many pinned bookmarks")
			}
			SqliteStorageError::DatabaseNotFound => {
				write!(f, "sqlite database was not found in this namespace branch")
			}
		}
	}
}

impl std::error::Error for SqliteStorageError {}
