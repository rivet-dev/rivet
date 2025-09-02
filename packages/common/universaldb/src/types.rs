use std::{fmt, ops::Deref};

use crate::FdbError;

pub struct TransactionCommitError {
	pub err: FdbError,
}

impl TransactionCommitError {
	pub fn new(err: FdbError) -> Self {
		Self { err }
	}

	pub fn code(&self) -> i32 {
		self.err.code()
	}
}

impl Deref for TransactionCommitError {
	type Target = FdbError;
	fn deref(&self) -> &FdbError {
		&self.err
	}
}

impl From<TransactionCommitError> for FdbError {
	fn from(tce: TransactionCommitError) -> FdbError {
		tce.err
	}
}

impl fmt::Debug for TransactionCommitError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "TransactionCommitError({})", self.err)
	}
}

impl fmt::Display for TransactionCommitError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.err.fmt(f)
	}
}

pub type TransactionCommitted = ();
pub type TransactionCancelled = ();

/// Indicates the transaction might have committed
#[derive(Debug, Clone, Copy)]
pub struct MaybeCommitted(pub bool);
