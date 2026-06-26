use std::{
	future::Future,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
};

use anyhow::{Context, Result, bail};
use futures_util::{StreamExt, stream};
use slatedb::{
	Db, DbSnapshot, IterationOrder,
	config::{DurabilityLevel, ReadOptions, ScanOptions, WriteOptions},
};
use tokio::sync::{Mutex, OnceCell};

use crate::{
	driver::TransactionDriver,
	error::DatabaseError,
	key_selector::KeySelector,
	options::{ConflictRangeType, MutationType},
	range_option::RangeOption,
	tx_ops::TransactionOperations,
	utils::IsolationLevel,
	value::{KeyValue, Slice, Value, Values},
};

use super::{
	commit::build_write_batch, transaction_conflict_tracker::TransactionConflictTracker,
};

#[derive(Clone)]
struct BeginState {
	read_version: u64,
	snapshot: Arc<DbSnapshot>,
}

pub struct SlateDbTransactionDriver {
	db: Arc<Db>,
	operations: TransactionOperations,
	committed: AtomicBool,
	txn_conflict_tracker: TransactionConflictTracker,
	commit_mutex: Arc<Mutex<()>>,
	last_applied_version: Arc<AtomicU64>,
	begin_state: OnceCell<BeginState>,
	active: Option<Arc<AtomicBool>>,
}

impl SlateDbTransactionDriver {
	pub fn new(
		db: Arc<Db>,
		txn_conflict_tracker: TransactionConflictTracker,
		commit_mutex: Arc<Mutex<()>>,
		last_applied_version: Arc<AtomicU64>,
		active: Option<Arc<AtomicBool>>,
	) -> Self {
		SlateDbTransactionDriver {
			db,
			operations: TransactionOperations::default(),
			committed: AtomicBool::new(false),
			txn_conflict_tracker,
			commit_mutex,
			last_applied_version,
			begin_state: OnceCell::new(),
			active,
		}
	}

	fn is_active(&self) -> bool {
		self.active
			.as_ref()
			.is_none_or(|active| active.load(Ordering::Acquire))
	}

	async fn begin(&self) -> Result<&BeginState> {
		self.begin_state
			.get_or_try_init(|| async {
				let _guard = self.commit_mutex.lock().await;
				let read_version = self.last_applied_version.load(Ordering::Acquire);
				let snapshot = self
					.db
					.snapshot()
					.await
					.context("failed to create SlateDB snapshot")?;
				anyhow::Ok(BeginState {
					read_version,
					snapshot,
				})
			})
			.await
	}

	fn read_options() -> ReadOptions {
		ReadOptions::new()
			.with_durability_filter(DurabilityLevel::Memory)
			.with_dirty(false)
	}

	fn scan_options(reverse: bool) -> ScanOptions {
		let order = if reverse {
			IterationOrder::Descending
		} else {
			IterationOrder::Ascending
		};
		ScanOptions::new()
			.with_durability_filter(DurabilityLevel::Memory)
			.with_dirty(false)
			.with_order(order)
	}

	async fn snapshot_get(&self, key: &[u8]) -> Result<Option<Slice>> {
		let begin = self.begin().await?;
		Ok(begin
			.snapshot
			.get_with_options(key, &Self::read_options())
			.await
			.context("failed to read SlateDB snapshot value")?
			.map(|value| value.to_vec().into()))
	}

	async fn snapshot_get_key(
		&self,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> Result<Option<Slice>> {
		let begin = self.begin().await?;

		match (or_equal, offset) {
			(false, 1) => {
				let mut iter = begin
					.snapshot
					.scan_with_options(key.to_vec().., &Self::scan_options(false))
					.await
					.context("failed to scan SlateDB first_greater_or_equal")?;
				Ok(iter
					.next()
					.await
					.context("failed to iterate SlateDB first_greater_or_equal")?
					.map(|kv| kv.key.to_vec().into()))
			}
			(true, 1) => {
				let mut iter = begin
					.snapshot
					.scan_with_options(key.to_vec().., &Self::scan_options(false))
					.await
					.context("failed to scan SlateDB first_greater_than")?;
				while let Some(kv) = iter
					.next()
					.await
					.context("failed to iterate SlateDB first_greater_than")?
				{
					if kv.key.as_ref() > key {
						return Ok(Some(kv.key.to_vec().into()));
					}
				}
				Ok(None)
			}
			(false, 0) => {
				let mut iter = begin
					.snapshot
					.scan_with_options(..key.to_vec(), &Self::scan_options(true))
					.await
					.context("failed to scan SlateDB last_less_than")?;
				Ok(iter
					.next()
					.await
					.context("failed to iterate SlateDB last_less_than")?
					.map(|kv| kv.key.to_vec().into()))
			}
			(true, 0) => {
				let mut iter = begin
					.snapshot
					.scan_with_options(..=key.to_vec(), &Self::scan_options(true))
					.await
					.context("failed to scan SlateDB last_less_or_equal")?;
				Ok(iter
					.next()
					.await
					.context("failed to iterate SlateDB last_less_or_equal")?
					.map(|kv| kv.key.to_vec().into()))
			}
			_ => bail!("invalid key selector offset"),
		}
	}

	async fn resolve_key_selector_for_range(
		&self,
		begin_state: &BeginState,
		key: &[u8],
		or_equal: bool,
		offset: i32,
	) -> Result<Vec<u8>> {
		match (or_equal, offset) {
			(false, 1) => {
				let mut iter = begin_state
					.snapshot
					.scan_with_options(key.to_vec().., &Self::scan_options(false))
					.await
					.context("failed to scan SlateDB range first_greater_or_equal")?;
				Ok(iter
					.next()
					.await
					.context("failed to iterate SlateDB range first_greater_or_equal")?
					.map(|kv| kv.key.to_vec())
					.unwrap_or_else(|| vec![0xff; 255]))
			}
			(true, 1) => {
				let mut iter = begin_state
					.snapshot
					.scan_with_options(key.to_vec().., &Self::scan_options(false))
					.await
					.context("failed to scan SlateDB range first_greater_than")?;
				while let Some(kv) = iter
					.next()
					.await
					.context("failed to iterate SlateDB range first_greater_than")?
				{
					if kv.key.as_ref() > key {
						return Ok(kv.key.to_vec());
					}
				}
				Ok(vec![0xff; 255])
			}
			_ => Ok(key.to_vec()),
		}
	}

	async fn snapshot_get_range(
		&self,
		opt: &RangeOption<'_>,
	) -> Result<Values> {
		let begin_state = self.begin().await?;
		let resolved_begin = self
			.resolve_key_selector_for_range(
				begin_state,
				opt.begin.key(),
				opt.begin.or_equal(),
				opt.begin.offset(),
			)
			.await?;
		let resolved_end = self
			.resolve_key_selector_for_range(
				begin_state,
				opt.end.key(),
				opt.end.or_equal(),
				opt.end.offset(),
			)
			.await?;

		if resolved_begin >= resolved_end {
			return Ok(Values::new(Vec::new()));
		}

		let mut results = Vec::new();
		let limit = opt.limit.unwrap_or(usize::MAX);
		let mut iter = begin_state
			.snapshot
			.scan_with_options(
				resolved_begin.clone()..resolved_end.clone(),
				&Self::scan_options(opt.reverse),
			)
			.await
			.context("failed to scan SlateDB range")?;

		while let Some(kv) = iter
			.next()
			.await
			.context("failed to iterate SlateDB range")?
		{
			results.push(KeyValue::new(kv.key.to_vec(), kv.value.to_vec()));
			if results.len() >= limit {
				break;
			}
		}

		Ok(Values::new(results))
	}

	async fn commit_inner(&self) -> Result<()> {
		if self.committed.load(Ordering::SeqCst) {
			return Ok(());
		}
		if !self.is_active() {
			return Err(DatabaseError::NotCommitted.into());
		}
		self.committed.store(true, Ordering::SeqCst);

		let begin = self.begin().await?.clone();
		let (operations, conflict_ranges) = self.operations.consume();

		let _guard = self.commit_mutex.lock().await;
		if !self.is_active() {
			return Err(DatabaseError::NotCommitted.into());
		}
		let Some(commit_version) = self
			.txn_conflict_tracker
			.check_and_insert(begin.read_version, conflict_ranges)
			.await
		else {
			return Err(DatabaseError::NotCommitted.into());
		};

		let batch = build_write_batch(self.db.clone(), operations).await?;
		if !batch.is_empty() {
			if let Err(error) = self
				.db
				.write_with_options(batch, &WriteOptions::default())
				.await
				.context("failed to write SlateDB batch")
			{
				if let Some(active) = &self.active {
					active.store(false, Ordering::Release);
					tracing::warn!(?error, "demoting SlateDB leader after write failure");
					return Err(DatabaseError::NotCommitted.into());
				}
				return Err(error);
			}
		}
		self.last_applied_version
			.store(commit_version, Ordering::Release);

		Ok(())
	}
}

impl TransactionDriver for SlateDbTransactionDriver {
	fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.operations.atomic_op(key, param, op_type);
	}

	fn get<'a>(
		&'a self,
		key: &[u8],
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Option<Slice>>> + Send + 'a>> {
		let key = key.to_vec();
		Box::pin(async move {
			self.operations
				.get_with_callback(&key, isolation_level, || async {
					self.snapshot_get(&key).await
				})
				.await
		})
	}

	fn get_key<'a>(
		&'a self,
		selector: &KeySelector<'a>,
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Slice>> + Send + 'a>> {
		let selector = selector.clone();
		Box::pin(async move {
			let key = selector.key().to_vec();
			let offset = selector.offset();
			let or_equal = selector.or_equal();
			self.operations
				.get_key(&selector, isolation_level, || async {
					Ok(self
						.snapshot_get_key(&key, or_equal, offset)
						.await?
						.unwrap_or_else(Slice::new))
				})
				.await
		})
	}

	fn get_range<'a>(
		&'a self,
		opt: &RangeOption<'a>,
		_iteration: usize,
		isolation_level: IsolationLevel,
	) -> Pin<Box<dyn Future<Output = Result<Values>> + Send + 'a>> {
		let opt = opt.clone();
		Box::pin(async move {
			self.operations
				.get_range(&opt, isolation_level, || async {
					self.snapshot_get_range(&opt).await
				})
				.await
		})
	}

	fn get_ranges_keyvalues<'a>(
		&'a self,
		opt: RangeOption<'a>,
		isolation_level: IsolationLevel,
	) -> crate::value::Stream<'a, Value> {
		let fut = async move {
			match self.get_range(&opt, 1, isolation_level).await {
				Ok(values) => values
					.into_iter()
					.map(|kv| Ok(Value::from_keyvalue(kv)))
					.collect::<Vec<_>>(),
				Err(e) => vec![Err(e)],
			}
		};

		Box::pin(stream::once(fut).flat_map(stream::iter))
	}

	fn set(&self, key: &[u8], value: &[u8]) {
		self.operations.set(key, value);
	}

	fn clear(&self, key: &[u8]) {
		self.operations.clear(key);
	}

	fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.operations.clear_range(begin, end);
	}

	fn commit(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		Box::pin(async move { self.commit_inner().await })
	}

	fn reset(&mut self) {
		self.operations.clear_all();
		self.committed.store(false, Ordering::SeqCst);
		self.begin_state.take();
	}

	fn cancel(&self) {
		self.operations.clear_all();
		self.committed.store(true, Ordering::SeqCst);
	}

	fn add_conflict_range(
		&self,
		begin: &[u8],
		end: &[u8],
		conflict_type: ConflictRangeType,
	) -> Result<()> {
		self.operations
			.add_conflict_range(begin, end, conflict_type);
		Ok(())
	}

	fn get_estimated_range_size_bytes<'a>(
		&'a self,
		begin: &'a [u8],
		end: &'a [u8],
	) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'a>> {
		let begin = begin.to_vec();
		let end = end.to_vec();
		Box::pin(async move {
			if begin >= end {
				return Ok(0);
			}

			let begin_state = self.begin().await?;
			let mut iter = begin_state
				.snapshot
				.scan_with_options(begin..end, &Self::scan_options(false))
				.await
				.context("failed to scan SlateDB range for size estimate")?;
			let mut total: i64 = 0;
			let mut count = 0usize;
			const MAX_KEYS: usize = 1024;
			while let Some(kv) = iter
				.next()
				.await
				.context("failed to iterate SlateDB range for size estimate")?
			{
				total = total.saturating_add((kv.key.len() + kv.value.len()) as i64);
				count += 1;
				if count >= MAX_KEYS {
					break;
				}
			}
			Ok(total)
		})
	}

	fn commit_ref(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
		Box::pin(async move { self.commit_inner().await })
	}
}
