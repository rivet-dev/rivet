use std::{
	future::Future,
	ops::Deref,
	pin::Pin,
	sync::{
		Arc, OnceLock,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result};
use futures_util::StreamExt;

use crate::{
	driver::TransactionDriver,
	key_selector::KeySelector,
	metrics,
	options::{ConflictRangeType, MutationType, Priority},
	range_option::RangeOption,
	throttle::{ThrottleCharge, ThrottleClass, ThrottleDecision, ThrottleKind, ThrottleTxn},
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

fn observe_keys(op: &'static str, keys: usize) {
	if keys > 0 {
		metrics::OPERATION_KEYS
			.with_label_values(&[op])
			.inc_by(u64::try_from(keys).unwrap_or(u64::MAX));
	}
}

/// Byte counters for one transaction attempt.
///
/// Shared across the clones `with_name`/`with_subspace` produce, so a caller sees the whole
/// transaction's cost rather than the part that happened to go through the handle it holds. Each
/// driver builds a fresh `Transaction` per attempt, so a retry starts these back at zero.
#[derive(Default)]
pub(crate) struct TxnCounters {
	/// Key plus value bytes every read operation has returned.
	read: AtomicU64,
	/// Key plus value bytes every write operation has submitted.
	write: AtomicU64,
	/// Write volume a caller reported by hand, most importantly the data a range clear removes.
	manual_write: AtomicU64,
	/// The throttle counter this attempt's reads charge, resolved once when the transaction opts in.
	/// Charging a read is then one atomic add on a counter this transaction already holds, with no
	/// map lookup on the read path.
	read_charge: OnceLock<Arc<AtomicU64>>,
}

#[derive(Clone)]
pub struct Transaction {
	pub(crate) driver: Arc<dyn TransactionDriver>,
	subspace: Subspace,
	name: &'static str,
	counters: Arc<TxnCounters>,
	/// Throttle bookkeeping, attached by the [`crate::Database`] this transaction came from.
	throttle: Option<Arc<ThrottleTxn>>,
}

impl Transaction {
	pub(crate) fn new(driver: Arc<dyn TransactionDriver>) -> Self {
		Transaction {
			driver: driver,
			subspace: tuple::Subspace::all().into(),
			name: DEFAULT_TXN_NAME,
			counters: Arc::new(TxnCounters::default()),
			throttle: None,
		}
	}

	pub(crate) fn with_name(&self, name: &'static str) -> Self {
		Transaction {
			driver: self.driver.clone(),
			subspace: self.subspace.clone(),
			name,
			counters: self.counters.clone(),
			throttle: self.throttle.clone(),
		}
	}

	pub(crate) fn with_throttle(&self, throttle: Arc<ThrottleTxn>) -> Self {
		Transaction {
			driver: self.driver.clone(),
			subspace: self.subspace.clone(),
			name: self.name,
			counters: self.counters.clone(),
			throttle: Some(throttle),
		}
	}

	/// Creates a new transaction instance with the provided subspace.
	pub fn with_subspace(&self, subspace: Subspace) -> Self {
		Transaction {
			driver: self.driver.clone(),
			subspace,
			name: self.name,
			counters: self.counters.clone(),
			throttle: self.throttle.clone(),
		}
	}

	/// Key plus value bytes this transaction has read from the database so far, across every read
	/// operation and every handle cloned from it.
	///
	/// This is the transaction's real read cost. A caller that needs to account for that cost (for
	/// example charging a rate limiter) must use this rather than measuring whatever subset of the
	/// rows it retained, because the reads a scan discards are load on the database all the same.
	pub fn read_bytes(&self) -> u64 {
		self.counters.read.load(Ordering::Relaxed)
	}

	/// Key plus value bytes this transaction has submitted to the database so far, across every write
	/// operation and every handle cloned from it.
	///
	/// This is what the operations carry, which is the whole cost of a `set` or an atomic mutation but
	/// not of a range clear, which submits two keys and removes an unbounded amount of data. A caller
	/// accounting for a range clear must report the removed volume with
	/// [`Transaction::charge_throttle_bytes`].
	pub fn write_bytes(&self) -> u64 {
		self.counters.write.load(Ordering::Relaxed)
	}

	/// What the throttle charges this attempt's writes: the operation bytes plus whatever the caller
	/// reported by hand. Manual volume is deliberately outside [`Transaction::write_bytes`], which
	/// stays a truthful measure of what the transaction submitted.
	pub(crate) fn throttled_write_bytes(&self) -> u64 {
		self.counters
			.write
			.load(Ordering::Relaxed)
			.saturating_add(self.counters.manual_write.load(Ordering::Relaxed))
	}

	/// Charges everything this transaction reads and writes against a named cluster-wide byte budget.
	///
	/// Call this at the top of the transaction body. The whole transaction is counted regardless of
	/// where the call sits: reads already made are charged when this runs, and later ones as they
	/// happen. Reads are charged for every attempt including ones that never commit, and writes are
	/// charged once, only if the transaction commits. See [`crate::throttle`] for why the two sides
	/// differ.
	///
	/// Opting in does not make a transaction yield. [`Transaction::check_throttle`] is what backs off;
	/// a transaction that charges without ever checking keeps the shared estimate honest while leaving
	/// itself ungated, which is what a control-plane pass that must not be denied wants.
	pub fn charge_throttle(&self, name: &'static str, charge: ThrottleCharge) -> Result<()> {
		let throttle = self
			.throttle
			.as_ref()
			.context("transaction has no throttle context; it was not created by a `Database`")?;
		throttle.register(name, charge)?;

		if charge.covers(ThrottleKind::Read)
			&& let Some(counter) = throttle.read_bucket(name)
		{
			// Reads issued before this call are charged here, so where the opt-in sits in the body
			// cannot silently drop volume.
			counter.fetch_add(
				self.counters.read.load(Ordering::Relaxed),
				Ordering::Relaxed,
			);
			let _ = self.counters.read_charge.set(counter);
		}

		Ok(())
	}

	/// Reports byte volume the operation sizes do not capture, most importantly the data a range clear
	/// removes. Charged on the same terms as the automatic counters.
	pub fn charge_throttle_bytes(&self, kind: ThrottleKind, bytes: u64) {
		match kind {
			ThrottleKind::Read => self.observe_read_bytes("manual", bytes as usize),
			ThrottleKind::Write => {
				self.counters
					.manual_write
					.fetch_add(bytes, Ordering::Relaxed);
			}
		}
	}

	/// Samples the admission gate for one named budget. Answers from process-local state without
	/// reading the database, so this is cheap enough to call before doing anything expensive.
	///
	/// An axis with no configured budget admits everything.
	pub fn check_throttle(
		&self,
		name: &'static str,
		kind: ThrottleKind,
		class: ThrottleClass,
	) -> Result<ThrottleDecision> {
		Ok(self
			.throttle
			.as_ref()
			.context("transaction has no throttle context; it was not created by a `Database`")?
			.state
			.check(name, kind, class))
	}

	/// The throttle context, for the [`crate::Database`] retry loop to charge attempts against.
	pub(crate) fn throttle(&self) -> Option<&Arc<ThrottleTxn>> {
		self.throttle.as_ref()
	}

	/// Records one operation's outcome and latency, warning on a slow one.
	fn observe_operation<T>(
		&self,
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
				txn_name = self.name,
				op,
				isolation,
				result,
				duration_ms = elapsed.as_millis() as u64,
				"slow udb operation"
			);
		}
	}

	/// Records read volume for one operation, against both the process-wide metric and this
	/// transaction's own counter, which [`Transaction::read_bytes`] reads back.
	fn observe_read_bytes(&self, op: &'static str, bytes: usize) {
		if bytes == 0 {
			return;
		}

		let bytes = bytes as u64;
		metrics::OPERATION_BYTES
			.with_label_values(&[op, "read", self.name])
			.inc_by(bytes);
		self.counters.read.fetch_add(bytes, Ordering::Relaxed);

		// One atomic add against the counter resolved at opt-in. Reads are charged as they happen
		// rather than at the end of the attempt, so a long scan is visible to the gate while it is
		// still running rather than only once it finishes.
		if let Some(counter) = self.counters.read_charge.get() {
			counter.fetch_add(bytes, Ordering::Relaxed);
		}
	}

	/// Records write volume for one operation, against both the process-wide metric and this
	/// transaction's own counter, which [`Transaction::write_bytes`] reads back.
	fn observe_write_bytes(&self, op: &'static str, bytes: usize) {
		if bytes == 0 {
			return;
		}

		let bytes = bytes as u64;
		metrics::OPERATION_BYTES
			.with_label_values(&[op, "write", self.name])
			.inc_by(bytes);
		self.counters.write.fetch_add(bytes, Ordering::Relaxed);
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
		async move {
			let result = self.driver.get(&key, isolation_level).await;
			self.observe_operation("get", isolation_label(isolation_level), start, &result);
			observe_keys("get", 1);
			if let Ok(Some(value)) = &result {
				self.observe_read_bytes("get", key_bytes + value.len());
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
		async move {
			let result = self.driver.get_key(selector, isolation_level).await;
			self.observe_operation("get_key", isolation_label(isolation_level), start, &result);
			observe_keys("get_key", 1);
			if let Ok(value) = &result {
				self.observe_read_bytes("get_key", value.len());
			}
			result
		}
	}

	/// Reads one page of a range, as FoundationDB's `get_range` does. It does not return the whole
	/// range: check `Values::more` and continue with `RangeOption::next_range` and the next
	/// `iteration`, or use [`Transaction::get_ranges_keyvalues`] to stream every row.
	pub fn get_range<'a, 'k>(
		&'a self,
		opt: &'k RangeOption<'k>,
		iteration: usize,
		isolation_level: IsolationLevel,
	) -> impl Future<Output = Result<Values>> + use<'a, 'k> {
		let start = std::time::Instant::now();
		async move {
			let result = self.driver.get_range(opt, iteration, isolation_level).await;
			self.observe_operation(
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
				self.observe_read_bytes("get_range", bytes);
			}
			result
		}
	}

	pub fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
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
							self.observe_read_bytes(
								"get_ranges_keyvalues",
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
		self.observe_write_bytes("set", key.len() + value.len());
		self.driver.set(key, value)
	}

	fn atomic_op_bytes(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		observe_keys("atomic_op", 1);
		self.observe_write_bytes("atomic_op", key.len() + param.len());
		self.driver.atomic_op(key, param, op_type)
	}

	pub fn clear(&self, key: &[u8]) {
		observe_keys("clear", 1);
		self.observe_write_bytes("clear", key.len());
		self.driver.clear(key)
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		observe_keys("clear_range", 2);
		self.observe_write_bytes("clear_range", begin.len() + end.len());
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
		self.observe_write_bytes("add_conflict_range", begin.len() + end.len());
		self.driver.add_conflict_range(begin, end, conflict_type)
	}

	pub fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		self.driver.get_estimated_range_size_bytes(begin, end)
	}

	/// Bytes this transaction would carry if it committed now, measured the way the database's own
	/// transaction size limit is.
	///
	/// Distinct from [`Transaction::write_bytes`], which counts only the keys and values the caller
	/// submitted. This includes what the database charges on top, above all the write conflict range
	/// every mutation adds, which for a transaction of many small writes is most of its size. A
	/// caller sizing itself against the transaction limit has to use this one.
	pub fn approximate_size<'a>(
		&'a self,
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		self.driver.approximate_size()
	}

	/// Adds a tag intended for throttling to the current transaction.
	pub fn tag(&self, tag: &str) -> Result<()> {
		self.driver.tag(tag)
	}

	pub fn priority(&self, priority: Priority) -> Result<()> {
		self.driver.priority(priority)
	}

	/// Caps how many times this transaction is retried inside [`crate::Database::txn`]. Once the cap
	/// is reached the retry loop stops and hands the most recent error back to the caller.
	///
	/// Set this on a transaction whose retries are not free to the rest of the system: the retry loop
	/// is internal, so a caller that wraps it sees one long call rather than N attempts, and work an
	/// aborted attempt did (reads it issued, budget it consumed) is invisible to anything measuring
	/// outside the loop. A low cap turns those retries back into separate calls the caller can
	/// observe and pace. Retrying is the right default everywhere else, so this is opt-in per
	/// transaction; the database-wide equivalent is [`crate::Database::txn_retry_limit`].
	///
	/// A limit of `0` disables retrying entirely, so the first retryable error is returned. Drivers
	/// that do not implement it ignore it and keep retrying.
	pub fn retry_limit(&self, limit: i32) -> Result<()> {
		self.driver.retry_limit(limit)
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

	/// Reads one page of a range, not the whole range. See [`Transaction::get_range`].
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

	pub(crate) fn with_throttle(&self, throttle: Arc<ThrottleTxn>) -> Self {
		RetryableTransaction {
			inner: self.inner.with_throttle(throttle),
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
