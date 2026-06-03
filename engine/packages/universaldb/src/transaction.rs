use std::{future::Future, ops::Deref, pin::Pin, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures_util::StreamExt;

use crate::{
	driver::TransactionDriver,
	key_selector::KeySelector,
	metrics,
	options::{ConflictRangeType, MutationType, Priority},
	range_option::RangeOption,
	tuple::{self, TuplePack, TupleUnpack},
	utils::{
		CherryPick, FormalKey, IsolationLevel, MaybeCommitted, OptSliceExt, Subspace,
		end_of_key_range,
	},
	value::{Slice, Value, Values},
};

pub const TXN_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_TXN_NAME: &str = "manual";
const SLOW_OPERATION_WARN_THRESHOLD: Duration = Duration::from_secs(1);

fn isolation_label(isolation_level: IsolationLevel) -> &'static str {
	match isolation_level {
		IsolationLevel::Serializable => "serializable",
		IsolationLevel::Snapshot => "snapshot",
	}
}

fn result_label<T>(result: &Result<T>) -> &'static str {
	if result.is_ok() { "ok" } else { "error" }
}

fn observe_operation<T>(
	txn_name: &'static str,
	op: &'static str,
	isolation: &'static str,
	start: std::time::Instant,
	result: &Result<T>,
) {
	let result = result_label(result);
	let elapsed = start.elapsed();
	metrics::OPERATION_TOTAL
		.with_label_values(&[op, isolation, result])
		.inc();
	metrics::OPERATION_DURATION
		.with_label_values(&[op, isolation, result])
		.observe(elapsed.as_secs_f64());
	if elapsed >= SLOW_OPERATION_WARN_THRESHOLD {
		tracing::warn!(
			txn_name,
			op,
			isolation,
			result,
			duration_ms = elapsed.as_millis() as u64,
			"slow udb operation"
		);
	}
}

fn observe_bytes(txn_name: &'static str, op: &'static str, direction: &'static str, bytes: usize) {
	if bytes == 0 {
		return;
	}

	let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
	metrics::OPERATION_BYTES
		.with_label_values(&[op, direction])
		.inc_by(bytes);
	match direction {
		"read" => metrics::TRANSACTION_READ_BYTES
			.with_label_values(&[txn_name])
			.inc_by(bytes),
		"write" => metrics::TRANSACTION_MUTATION_BYTES
			.with_label_values(&[txn_name])
			.inc_by(bytes),
		_ => {}
	}
}

fn observe_keys(op: &'static str, keys: usize) {
	if keys > 0 {
		metrics::OPERATION_KEYS
			.with_label_values(&[op])
			.inc_by(u64::try_from(keys).unwrap_or(u64::MAX));
	}
}

#[derive(Clone)]
pub struct Transaction {
	pub(crate) driver: Arc<dyn TransactionDriver>,
	subspace: Subspace,
	name: &'static str,
}

impl Transaction {
	pub(crate) fn new(driver: Arc<dyn TransactionDriver>) -> Self {
		Transaction {
			driver: driver,
			subspace: tuple::Subspace::all().into(),
			name: DEFAULT_TXN_NAME,
		}
	}

	pub(crate) fn with_name(&self, name: &'static str) -> Self {
		Transaction {
			driver: self.driver.clone(),
			subspace: self.subspace.clone(),
			name,
		}
	}

	/// Creates a new transaction instance with the provided subspace.
	pub fn with_subspace(&self, subspace: Subspace) -> Self {
		Transaction {
			driver: self.driver.clone(),
			subspace,
			name: self.name,
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
		self.subspace.unpack(key).with_context(|| {
			format!(
				"failed unpacking key {} as {}",
				hex::encode(key),
				std::any::type_name::<T>(),
			)
		})
	}

	pub fn write<T: FormalKey + TuplePack>(&self, key: &T, value: T::Value) -> Result<()> {
		self.set(
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
		self.get(&self.subspace.pack(key), isolation_level)
			.await?
			.read(key)
	}

	pub async fn read_opt<'de, T: FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		isolation_level: IsolationLevel,
	) -> Result<Option<T::Value>> {
		self.get(&self.subspace.pack(key), isolation_level)
			.await?
			.read_opt(key)
	}

	pub async fn exists<T: TuplePack>(
		&self,
		key: &T,
		isolation_level: IsolationLevel,
	) -> Result<bool> {
		Ok(self
			.get(&self.subspace.pack(key), isolation_level)
			.await?
			.is_some())
	}

	pub fn delete<T: TuplePack>(&self, key: &T) {
		self.clear(&self.subspace.pack(key));
	}

	pub fn delete_subspace(&self, subspace: &Subspace) {
		self.informal()
			.clear_subspace_range(&self.subspace.join(&subspace));
	}

	pub fn delete_key_subspace<T: TuplePack>(&self, key: &T) {
		self.informal()
			.clear_subspace_range(&self.subspace.subspace(&key));
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

		self.add_conflict_range(&key_buf, &end_of_key_range(&key_buf), conflict_type)
	}

	pub fn atomic_op<'de, T: std::fmt::Debug + FormalKey + TuplePack + TupleUnpack<'de>>(
		&self,
		key: &'de T,
		param: &[u8],
		op_type: MutationType,
	) {
		self.atomic_op_bytes(&self.subspace.pack(key), param, op_type)
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
		self.get_ranges_keyvalues(opt, isolation_level)
	}

	// TODO: Fix types
	// pub fn read_entries<'a, T: FormalKey + for<'de> TupleUnpack<'de>>(
	// 	&'a self,
	// 	opt: RangeOption<'a>,
	// 	isolation_level: IsolationLevel,
	// ) -> impl futures_util::Stream<Item = Result<(T, T::Value)>> {
	// 	self.read_range(opt, isolation_level)
	// 		.map(|res| self.read_entry(&res?))
	// }

	// ==== TODO: Remove. all of these should only be used via `tx.informal()` ====
	pub fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Option<Slice>>> + 'a {
		let start = std::time::Instant::now();
		let key = key.to_vec();
		let key_bytes = key.len();
		let txn_name = self.name;
		async move {
			let result = self.driver.get(&key, isolation_level).await;
			observe_operation(
				txn_name,
				"get",
				isolation_label(isolation_level),
				start,
				&result,
			);
			observe_keys("get", 1);
			if let Ok(Some(value)) = &result {
				observe_bytes(txn_name, "get", "read", key_bytes + value.len());
			}
			result
		}
	}

	pub fn get_key<'a, 'k>(
		&'a self,
		selector: &'k KeySelector<'k>,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Slice>> + use<'a, 'k> {
		let start = std::time::Instant::now();
		let txn_name = self.name;
		async move {
			let result = self.driver.get_key(selector, isolation_level).await;
			observe_operation(
				txn_name,
				"get_key",
				isolation_label(isolation_level),
				start,
				&result,
			);
			observe_keys("get_key", 1);
			if let Ok(value) = &result {
				observe_bytes(txn_name, "get_key", "read", value.len());
			}
			result
		}
	}

	pub fn get_range<'a, 'k>(
		&'a self,
		opt: &'k RangeOption<'k>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Values>> + use<'a, 'k> {
		let start = std::time::Instant::now();
		let txn_name = self.name;
		async move {
			let result = self.driver.get_range(opt, iteration, isolation_level).await;
			observe_operation(
				txn_name,
				"get_range",
				isolation_label(isolation_level),
				start,
				&result,
			);
			if let Ok(values) = &result {
				observe_keys("get_range", values.len());
				let bytes = values
					.iter()
					.map(|value| value.key().len() + value.value().len())
					.sum();
				observe_bytes(txn_name, "get_range", "read", bytes);
			}
			result
		}
	}

	pub fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		let txn_name = self.name;
		let isolation = isolation_label(isolation_level);
		metrics::OPERATION_TOTAL
			.with_label_values(&["get_ranges_keyvalues", isolation, "stream"])
			.inc();
		Box::pin(
			self.driver
				.get_ranges_keyvalues(opt, isolation_level)
				.map(move |result| {
					match &result {
						Ok(value) => {
							observe_keys("get_ranges_keyvalues", 1);
							observe_bytes(
								txn_name,
								"get_ranges_keyvalues",
								"read",
								value.key().len() + value.value().len(),
							);
						}
						Err(_) => {
							metrics::OPERATION_TOTAL
								.with_label_values(&["get_ranges_keyvalues", isolation, "error"])
								.inc();
						}
					}
					result
				}),
		)
	}

	pub fn set(&self, key: &[u8], value: &[u8]) {
		observe_keys("set", 1);
		observe_bytes(self.name, "set", "write", key.len() + value.len());
		self.driver.set(key, value)
	}

	fn atomic_op_bytes(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		observe_keys("atomic_op", 1);
		observe_bytes(self.name, "atomic_op", "write", key.len() + param.len());
		self.driver.atomic_op(key, param, op_type)
	}

	pub fn clear(&self, key: &[u8]) {
		observe_keys("clear", 1);
		observe_bytes(self.name, "clear", "write", key.len());
		self.driver.clear(key)
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		observe_keys("clear_range", 2);
		observe_bytes(self.name, "clear_range", "write", begin.len() + end.len());
		self.driver.clear_range(begin, end)
	}

	pub fn clear_subspace_range(&self, subspace: &tuple::Subspace) {
		let (begin, end) = subspace.range();
		self.clear_range(&begin, &end);
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
		observe_keys("add_conflict_range", 2);
		observe_bytes(
			self.name,
			"add_conflict_range",
			"write",
			begin.len() + end.len(),
		);
		self.driver.add_conflict_range(begin, end, conflict_type)
	}

	pub fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		self.driver.get_estimated_range_size_bytes(begin, end)
	}

	/// Adds a tag intended for throttling to the current transaction.
	pub fn tag(&self, tag: &str) -> Result<()> {
		self.driver.tag(tag)
	}

	pub fn priority(&self, priority: Priority) -> Result<()> {
		self.driver.priority(priority)
	}
}

pub struct InformalTransaction<'t> {
	inner: &'t Transaction,
}

impl<'t> InformalTransaction<'t> {
	pub fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.inner.atomic_op_bytes(key, param, op_type)
	}

	// Read operations
	pub fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Option<Slice>>> + 'a {
		self.inner.get(key, isolation_level)
	}

	pub fn get_key<'a, 'k>(
		&'a self,
		selector: &'k KeySelector<'k>,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Slice>> + use<'a, 'k> {
		self.inner.get_key(selector, isolation_level)
	}

	pub fn get_range<'a, 'k>(
		&'a self,
		opt: &'k RangeOption<'k>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Values>> + use<'a, 'k> {
		self.inner.get_range(opt, iteration, isolation_level)
	}

	pub fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		self.inner.get_ranges_keyvalues(opt, isolation_level)
	}

	// Write operations
	pub fn set(&self, key: &[u8], value: &[u8]) {
		self.inner.set(key, value)
	}

	pub fn clear(&self, key: &[u8]) {
		self.inner.clear(key)
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.inner.clear_range(begin, end)
	}

	/// Clear all keys in a subspace range
	pub fn clear_subspace_range(&self, subspace: &tuple::Subspace) {
		let (begin, end) = subspace.range();
		self.inner.clear_range(&begin, &end);
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
		self.inner.add_conflict_range(begin, end, conflict_type)
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

	pub(crate) fn with_name(&self, name: &'static str) -> Self {
		RetryableTransaction {
			inner: self.inner.with_name(name),
			maybe_committed: self.maybe_committed,
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
