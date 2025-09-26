use std::{future::Future, ops::Deref, pin::Pin, sync::Arc};

use anyhow::{Context, Result};
use futures_util::StreamExt;

use crate::{
	driver::TransactionDriver,
	key_selector::KeySelector,
	options::{ConflictRangeType, MutationType},
	range_option::RangeOption,
	tuple::{self, TuplePack, TupleUnpack},
	utils::{
		CherryPick, FormalKey, IsolationLevel, MaybeCommitted, OptSliceExt, Subspace,
		end_of_key_range,
	},
	value::{Slice, Value, Values},
};

#[derive(Clone)]
pub struct Transaction {
	pub(crate) driver: Arc<dyn TransactionDriver>,
	subspace: Subspace,
}

impl Transaction {
	pub(crate) fn new(driver: Arc<dyn TransactionDriver>) -> Self {
		Transaction {
			driver: driver,
			subspace: tuple::Subspace::all().into(),
		}
	}

	/// Creates a new transaction instance with the provided subspace.
	pub fn with_subspace(&self, subspace: Subspace) -> Self {
		Transaction {
			driver: self.driver.clone(),
			subspace,
		}
	}

	pub fn informal(&self) -> InformalTransaction<'_> {
		InformalTransaction { inner: self }
	}

	pub fn pack<T: TuplePack>(&self, t: &T) -> Vec<u8> {
		self.subspace.pack(t)
	}

	/// Unpacks a key based on the subspace of this transaction.
	pub fn unpack<'de, T: TupleUnpack<'de>>(&self, key: &'de [u8]) -> Result<T> {
		self.subspace
			.unpack(key)
			.with_context(|| format!("failed unpacking key of {}", std::any::type_name::<T>()))
	}

	pub fn write<T: FormalKey + TuplePack>(&self, key: &T, value: T::Value) -> Result<()> {
		self.driver.set(
			&self.subspace.pack(key),
			&key.serialize(value).with_context(|| {
				format!(
					"failed serializing key value of {}",
					std::any::type_name::<T>(),
				)
			})?,
		);

		Ok(())
	}

	pub async fn read<'de, T: FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		isolation_level: IsolationLevel,
	) -> Result<T::Value> {
		self.driver
			.get(&self.subspace.pack(key), isolation_level)
			.await?
			.read(key)
	}

	pub async fn read_opt<'de, T: FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		isolation_level: IsolationLevel,
	) -> Result<Option<T::Value>> {
		self.driver
			.get(&self.subspace.pack(key), isolation_level)
			.await?
			.read_opt(key)
	}

	pub async fn exists<T: TuplePack>(
		&self,
		key: &T,
		isolation_level: IsolationLevel,
	) -> Result<bool> {
		Ok(self
			.driver
			.get(&self.subspace.pack(key), isolation_level)
			.await?
			.is_some())
	}

	pub fn delete<T: TuplePack>(&self, key: &T) {
		self.driver.clear(&self.subspace.pack(key));
	}

	pub fn delete_key_subspace<T: TuplePack>(&self, key: &T) {
		self.informal()
			.clear_subspace_range(&self.subspace.subspace(&self.subspace.pack(key)));
	}

	pub fn read_entry<T: FormalKey + for<'de> TupleUnpack<'de>>(
		&self,
		entry: &Value,
	) -> Result<(T, T::Value)> {
		let key = self.unpack::<T>(entry.key())?;
		let value = key.deserialize(entry.value()).with_context(|| {
			format!(
				"failed deserializing key value of {}",
				std::any::type_name::<T>()
			)
		})?;

		Ok((key, value))
	}

	pub async fn cherry_pick<T: CherryPick>(
		&self,
		subspace: impl TuplePack + Send,
		isolation_level: IsolationLevel,
	) -> Result<T::Output> {
		T::cherry_pick(self, subspace, isolation_level).await
	}

	pub fn add_conflict_key<T: TuplePack>(
		&self,
		key: &T,
		conflict_type: ConflictRangeType,
	) -> Result<()> {
		let key_buf = self.subspace.pack(key);

		self.driver
			.add_conflict_range(&key_buf, &end_of_key_range(&key_buf), conflict_type)
			.map_err(Into::into)
	}

	pub fn atomic_op<'de, T: FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		param: &[u8],
		op_type: MutationType,
	) {
		self.driver
			.atomic_op(&self.subspace.pack(key), param, op_type)
	}

	pub fn read_range<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		let opt = RangeOption {
			begin: KeySelector::new(
				[self.subspace.bytes(), opt.begin.key()].concat().into(),
				opt.begin.or_equal(),
				opt.begin.offset(),
			),
			end: KeySelector::new(
				[self.subspace.bytes(), opt.end.key()].concat().into(),
				opt.end.or_equal(),
				opt.end.offset(),
			),
			..opt
		};
		self.driver.get_ranges_keyvalues(opt, isolation_level)
	}

	pub fn read_entries<'a, T: FormalKey + for<'de> TupleUnpack<'de>>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> impl futures_util::Stream<Item = Result<(T, T::Value)>> {
		self.driver
			.get_ranges_keyvalues(opt, isolation_level)
			.map(|res| self.read_entry(&res?))
	}

	// ==== TODO: Remove. all of these should only be used via `tx.informal()` ====
	pub fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Option<Slice>>> + 'a {
		self.driver.get(key, isolation_level)
	}

	pub fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Slice>> + 'a {
		self.driver.get_key(selector, isolation_level)
	}

	pub fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Values>> + 'a {
		self.driver.get_range(opt, iteration, isolation_level)
	}

	pub fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		self.driver.get_ranges_keyvalues(opt, isolation_level)
	}

	pub fn set(&self, key: &[u8], value: &[u8]) {
		self.driver.set(key, value)
	}

	pub fn clear(&self, key: &[u8]) {
		self.driver.clear(key)
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.driver.clear_range(begin, end)
	}

	pub fn clear_subspace_range(&self, subspace: &tuple::Subspace) {
		let (begin, end) = subspace.range();
		self.driver.clear_range(&begin, &end);
	}

	pub fn cancel(&self) {
		self.driver.cancel()
	}

	pub fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> Result<()> {
		self.driver.add_conflict_range(begin, end, conflict_type)
	}

	pub fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		self.driver.get_estimated_range_size_bytes(begin, end)
	}
}

pub struct InformalTransaction<'t> {
	inner: &'t Transaction,
}

impl<'t> InformalTransaction<'t> {
	pub fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.inner.driver.atomic_op(key, param, op_type)
	}

	// Read operations
	pub fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Option<Slice>>> + 'a {
		self.inner.driver.get(key, isolation_level)
	}

	pub fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Slice>> + 'a {
		self.inner.driver.get_key(selector, isolation_level)
	}

	pub fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Values>> + 'a {
		self.inner.driver.get_range(opt, iteration, isolation_level)
	}

	pub fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		self.inner.driver.get_ranges_keyvalues(opt, isolation_level)
	}

	// Write operations
	pub fn set(&self, key: &[u8], value: &[u8]) {
		self.inner.driver.set(key, value)
	}

	pub fn clear(&self, key: &[u8]) {
		self.inner.driver.clear(key)
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.inner.driver.clear_range(begin, end)
	}

	/// Clear all keys in a subspace range
	pub fn clear_subspace_range(&self, subspace: &Subspace) {
		let (begin, end) = subspace.range();
		self.inner.driver.clear_range(&begin, &end);
	}

	pub fn cancel(&self) {
		self.inner.driver.cancel()
	}

	pub fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> Result<()> {
		self.inner
			.driver
			.add_conflict_range(begin, end, conflict_type)
	}

	pub fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		self.inner.driver.get_estimated_range_size_bytes(begin, end)
	}
}

/// Retryable transaction wrapper
#[derive(Clone)]
pub struct RetryableTransaction {
	pub(crate) inner: Transaction,
	pub(crate) maybe_committed: MaybeCommitted,
}

impl RetryableTransaction {
	pub fn new(transaction: Transaction) -> Self {
		RetryableTransaction {
			inner: transaction,
			maybe_committed: MaybeCommitted(false),
		}
	}

	pub fn maybe_committed(&self) -> MaybeCommitted {
		self.maybe_committed
	}
}

impl Deref for RetryableTransaction {
	type Target = Transaction;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
