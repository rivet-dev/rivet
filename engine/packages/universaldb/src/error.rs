#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
	#[error("transaction not committed due to conflict with another transaction")]
	NotCommitted,

	// TODO: Implement in rocksdb and postgres drivers
	#[error("transaction is too old to perform reads or be committed")]
	TransactionTooOld,

	#[error("transaction is too large: {actual_size_bytes} bytes, limit is {max_size_bytes} bytes")]
	TransactionTooLarge {
		actual_size_bytes: usize,
		max_size_bytes: usize,
	},

	#[error("key is too large: {actual_size_bytes} bytes, limit is {max_size_bytes} bytes")]
	KeyTooLarge {
		actual_size_bytes: usize,
		max_size_bytes: usize,
	},

	#[error("value is too large: {actual_size_bytes} bytes, limit is {max_size_bytes} bytes")]
	ValueTooLarge {
		actual_size_bytes: usize,
		max_size_bytes: usize,
	},

	#[error("max number of transaction retries reached")]
	MaxRetriesReached,

	#[error("operation issued while a commit was outstanding")]
	UsedDuringCommit,
}

impl DatabaseError {
	pub fn is_retryable(&self) -> bool {
		use DatabaseError::*;

		match self {
			NotCommitted | TransactionTooOld | MaxRetriesReached => true,
			_ => false,
		}
	}

	pub fn is_maybe_committed(&self) -> bool {
		false
	}
}
