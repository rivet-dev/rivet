#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
	#[error("transaction not committed due to conflict with another transaction")]
	NotCommitted,

	// TODO: Implement in rocksdb and postgres drivers
	#[error("transaction is too old to perform reads or be committed")]
	TransactionTooOld,

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
