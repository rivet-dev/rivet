use std::{future::Future, ops::Deref, pin::Pin, sync::Arc};

use crate::{
	FdbResult, KeySelector, RangeOption,
	driver::TransactionDriver,
	future::{FdbSlice, FdbValues},
	options::{ConflictRangeType, MutationType},
	tuple::Subspace,
	types::{TransactionCommitError, TransactionCommitted},
};

pub struct Transaction {
	pub(crate) driver: Box<dyn TransactionDriver>,
}

impl Transaction {
	pub(crate) fn new(driver: Box<dyn TransactionDriver>) -> Self {
		Transaction { driver: driver }
	}

	pub fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.driver.atomic_op(key, param, op_type)
	}

	// Read operations
	pub fn get<'a>(
		&'a self,
		key: &[u8],
		snapshot: bool,
	) -> impl Future<Output = FdbResult<Option<FdbSlice>>> + 'a {
		self.driver.get(key, snapshot)
	}

	pub fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		snapshot: bool,
	) -> impl Future<Output = FdbResult<FdbSlice>> + 'a {
		self.driver.get_key(selector, snapshot)
	}

	pub fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		snapshot: bool,
	) -> impl Future<Output = FdbResult<FdbValues>> + 'a {
		self.driver.get_range(opt, iteration, snapshot)
	}

	pub fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		snapshot: bool,
	) -> crate::future::FdbStream<'a, crate::future::FdbValue> {
		self.driver.get_ranges_keyvalues(opt, snapshot)
	}

	// Write operations
	pub fn set(&self, key: &[u8], value: &[u8]) {
		self.driver.set(key, value)
	}

	pub fn clear(&self, key: &[u8]) {
		self.driver.clear(key)
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.driver.clear_range(begin, end)
	}

	/// Clear all keys in a subspace range
	pub fn clear_subspace_range(&self, subspace: &Subspace) {
		let (begin, end) = subspace.range();
		self.clear_range(&begin, &end);
	}

	pub fn commit(
		self: Box<Self>,
	) -> Pin<Box<dyn Future<Output = Result<TransactionCommitted, TransactionCommitError>> + Send>>
	{
		self.driver.commit()
	}

	pub fn cancel(&self) {
		self.driver.cancel()
	}

	pub fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> FdbResult<()> {
		self.driver.add_conflict_range(begin, end, conflict_type)
	}

	pub fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = FdbResult<i64>> + Send + 'a>> {
		self.driver.get_estimated_range_size_bytes(begin, end)
	}
}

/// Retryable transaction wrapper
#[derive(Clone)]
pub struct RetryableTransaction {
	pub(crate) inner: Arc<Transaction>,
}

impl RetryableTransaction {
	pub fn new(transaction: Transaction) -> Self {
		RetryableTransaction {
			inner: Arc::new(transaction),
		}
	}
}

impl Deref for RetryableTransaction {
	type Target = Transaction;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl RetryableTransaction {
	/// Clear all keys in a subspace range
	pub fn clear_subspace_range(&self, subspace: &Subspace) {
		let (begin, end) = subspace.range();
		self.inner.clear_range(&begin, &end);
	}
}
