use std::{
	collections::BTreeMap,
	sync::{Mutex, MutexGuard},
};

use anyhow::Result;

use crate::{
	atomic::apply_atomic_op,
	key_selector::KeySelector,
	options::{ConflictRangeType, MutationType},
	range_option::RangeOption,
	utils::{IsolationLevel, end_of_key_range},
	value::{KeyValue, Slice, Values},
};

#[derive(Debug, Clone)]
pub enum Operation {
	SetValue {
		key: Vec<u8>,
		value: Vec<u8>,
	},
	Clear {
		key: Vec<u8>,
	},
	ClearRange {
		begin: Vec<u8>,
		end: Vec<u8>,
	},
	AtomicOp {
		key: Vec<u8>,
		param: Vec<u8>,
		op_type: MutationType,
	},
}

/// Per-entry overhead FoundationDB charges for a mutation, beyond the key and value it carries.
///
/// See [`approximate_size`] for where these two numbers come from and what they are worth.
const MUTATION_FRAMING_BYTES: i64 = 44;

/// Per-entry overhead FoundationDB charges for a conflict range, beyond its two boundary keys.
const CONFLICT_RANGE_FRAMING_BYTES: i64 = 32;

/// Approximates the transaction size FoundationDB would charge for these mutations and conflict
/// ranges, for drivers that have no such accounting of their own.
///
/// This is what the 10 MB transaction limit is measured against, and it is nothing like the keys and
/// values a caller submitted. Every `set` also adds an implicit write conflict range, and each entry
/// carries a fixed per-entry cost that dwarfs a small key, so a transaction of many small writes
/// costs several times what its contents suggest.
///
/// The two framing constants are calibrated against the one point where the limit has actually been
/// observed: depot's staged commit finalize, which writes a 29 byte key and an 8 byte value per
/// page, publishes at 60,146 pages on a live cluster and fails with a non-retryable
/// `transaction_too_large` at 62,152. That places the limit near 172 bytes per page against 37 bytes
/// of key and value, and the constants are set so this function reproduces it.
///
/// So this is a calibrated model, not an accounting. On FoundationDB the cluster's own figure is
/// authoritative and this function is not used. It exists so a bound measured on the FileSystem or
/// Postgres drivers means roughly the same thing, and it is deliberately shaped to over-estimate for
/// keys longer than the calibration point rather than under-estimate.
fn approximate_size(
	operations: &[Operation],
	conflict_ranges: &[(Vec<u8>, Vec<u8>, ConflictRangeType)],
) -> i64 {
	let mut size = 0i64;
	for operation in operations {
		let (key_len, payload_len) = match operation {
			Operation::SetValue { key, value } => (key.len(), value.len()),
			Operation::Clear { key } => (key.len(), 0),
			Operation::ClearRange { begin, end } => (begin.len(), end.len()),
			Operation::AtomicOp { key, param, .. } => (key.len(), param.len()),
		};
		size += key_len as i64 + payload_len as i64 + MUTATION_FRAMING_BYTES;
	}
	for (begin, end, _) in conflict_ranges {
		size += begin.len() as i64 + end.len() as i64 + CONFLICT_RANGE_FRAMING_BYTES;
	}
	size
}

/// Whether a transaction has nothing to commit: no mutations and no explicitly added write conflict
/// ranges. FDB never commits a read-only transaction, so it can never conflict and never causes
/// another transaction to conflict. Drivers use this to skip the commit path entirely.
pub fn is_read_only(
	operations: &[Operation],
	conflict_ranges: &[(Vec<u8>, Vec<u8>, ConflictRangeType)],
) -> bool {
	operations.is_empty()
		&& !conflict_ranges
			.iter()
			.any(|(_, _, conflict_type)| match conflict_type {
				ConflictRangeType::Write => true,
				ConflictRangeType::Read => false,
			})
}

#[derive(Debug, Clone)]
pub enum GetOutput {
	Value(Vec<u8>),
	Cleared,
	None,
	/// Indicates that atomic operations were found and need database value to resolve
	ApplyAtomicOps(Vec<(Vec<u8>, MutationType)>), // (param, op_type) pairs
}

#[derive(Default)]
pub struct TransactionOperations {
	operations: Mutex<Vec<Operation>>,
	conflict_ranges: Mutex<Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>>,
}

impl TransactionOperations {
	pub fn consume(&self) -> (Vec<Operation>, Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>) {
		(
			std::mem::take(&mut self.operations.lock().unwrap()),
			std::mem::take(&mut self.conflict_ranges.lock().unwrap()),
		)
	}

	pub fn add_operation(&self, operation: Operation) {
		self.operations.lock().unwrap().push(operation);
	}

	pub fn operations(&self) -> MutexGuard<'_, Vec<Operation>> {
		self.operations.lock().unwrap()
	}

	/// See [`approximate_size`].
	pub fn approximate_size(&self) -> i64 {
		approximate_size(
			&self.operations.lock().unwrap(),
			&self.conflict_ranges.lock().unwrap(),
		)
	}

	pub fn set(&self, key: &[u8], value: &[u8]) {
		self.add_conflict_range(key, &end_of_key_range(key), ConflictRangeType::Write);

		self.add_operation(Operation::SetValue {
			key: key.to_vec(),
			value: value.to_vec(),
		});
	}

	pub fn clear(&self, key: &[u8]) {
		self.add_conflict_range(key, &end_of_key_range(key), ConflictRangeType::Write);

		self.add_operation(Operation::Clear { key: key.to_vec() });
	}

	pub fn clear_range(&self, begin: &[u8], end: &[u8]) {
		self.add_conflict_range(begin, end, ConflictRangeType::Write);

		self.add_operation(Operation::ClearRange {
			begin: begin.to_vec(),
			end: end.to_vec(),
		});
	}

	pub fn atomic_op(&self, key: &[u8], param: &[u8], op_type: MutationType) {
		self.add_conflict_range(key, &end_of_key_range(key), ConflictRangeType::Write);

		self.add_operation(Operation::AtomicOp {
			key: key.to_vec(),
			param: param.to_vec(),
			op_type,
		});
	}

	pub fn get(&self, key: &[u8], isolation_level: IsolationLevel) -> GetOutput {
		if let IsolationLevel::Serializable = isolation_level {
			self.add_conflict_range(key, &end_of_key_range(key), ConflictRangeType::Read);
		}

		let mut atomic_ops: Vec<(Vec<u8>, MutationType)> = Vec::new();

		// Iterate through operations in reverse order to find the most recent operation for this key
		for op in self.operations().iter().rev() {
			match op {
				Operation::SetValue {
					key: set_key,
					value,
				} if set_key.as_slice() == key => {
					let mut result_value = Some(value.clone());

					// If we found atomic ops after this set, apply them to this value
					if !atomic_ops.is_empty() {
						// Apply atomic operations in forward order (reverse of how we collected them)
						for (param, op_type) in atomic_ops.into_iter().rev() {
							result_value =
								apply_atomic_op(result_value.as_deref(), &param, op_type);
						}
					}

					return result_value
						.map(GetOutput::Value)
						.unwrap_or(GetOutput::Cleared);
				}
				Operation::Clear { key: cleared_key } if cleared_key.as_slice() == key => {
					return GetOutput::Cleared;
				}
				Operation::ClearRange { begin, end }
					if key >= begin.as_slice() && key < end.as_slice() =>
				{
					return GetOutput::Cleared;
				}
				Operation::AtomicOp {
					key: atomic_key,
					param,
					op_type,
				} if atomic_key.as_slice() == key => {
					atomic_ops.push((param.clone(), *op_type));
				}
				_ => {}
			}
		}

		// If we found atomic operations but no set/clear, we need the database value
		if !atomic_ops.is_empty() {
			// Reverse to get operations in forward order
			atomic_ops.reverse();
			GetOutput::ApplyAtomicOps(atomic_ops)
		} else {
			GetOutput::None
		}
	}

	pub async fn get_with_callback<F, Fut>(
		&self,
		key: &[u8],
		isolation_level: IsolationLevel,
		get_from_db: F,
	) -> Result<Option<Slice>>
	where
		F: FnOnce() -> Fut,
		Fut: std::future::Future<Output = Result<Option<Slice>>>,
	{
		// Check local operations first
		match self.get(key, isolation_level) {
			GetOutput::Value(value) => Ok(Some(value.into())),
			GetOutput::Cleared => Ok(None),
			GetOutput::None => {
				// Fall back to database
				get_from_db().await
			}
			GetOutput::ApplyAtomicOps(atomic_ops) => {
				// Get the current value from database and apply atomic operations
				let db_value = get_from_db().await?;
				let mut result_value = db_value;

				// Apply all atomic operations in order
				for (param, op_type) in atomic_ops {
					result_value = apply_atomic_op(
						result_value.as_ref().map(|x| x.as_slice()),
						&param,
						op_type,
					)
					.map(Into::into);
				}

				Ok(result_value)
			}
		}
	}

	pub async fn get_key<F, Fut>(
		&self,
		selector: &KeySelector<'_>,
		isolation_level: IsolationLevel,
		get_from_db: F,
	) -> Result<Slice>
	where
		F: FnOnce() -> Fut,
		Fut: std::future::Future<Output = Result<Slice>>,
	{
		// Get the database result first
		let db_key = get_from_db().await?;

		// If there are no local operations, just return the database result
		if self.operations().is_empty() {
			// Add conflict range on resolved key
			if let IsolationLevel::Serializable = isolation_level {
				self.add_conflict_range(
					&db_key,
					&end_of_key_range(&db_key),
					ConflictRangeType::Read,
				);
			}

			return Ok(db_key);
		}

		// Check if db_key is cleared locally
		let db_key_cleared =
			!db_key.is_empty() && matches!(self.get(&db_key, isolation_level), GetOutput::Cleared);

		// Build a map of all local keys that currently exist (not cleared)
		let mut local_keys = BTreeMap::new();

		for op in &*self.operations() {
			match op {
				Operation::SetValue { key, .. } => {
					local_keys.insert(key.clone(), ());
				}
				Operation::Clear { key } => {
					local_keys.remove(key);
				}
				Operation::ClearRange { begin, end } => {
					let keys_to_remove: Vec<_> = local_keys
						.range(begin.clone()..end.clone())
						.map(|(k, _)| k.clone())
						.collect();
					for key in keys_to_remove {
						local_keys.remove(&key);
					}
				}
				// TODO: When MutationType::SetVersionstampedKey is implemented, fix
				Operation::AtomicOp { .. } => {}
			}
		}

		let search_key = selector.key().to_vec();
		let is_forward = selector.offset() >= 1;
		let include_equal = !selector.or_equal();

		// Find the best local key based on selector direction
		let best_local = if is_forward {
			// Looking for first key >= or > search_key
			if include_equal {
				local_keys
					.range(search_key.clone()..)
					.next()
					.map(|(k, _)| k.clone())
			} else {
				local_keys
					.range((
						std::ops::Bound::Excluded(search_key.clone()),
						std::ops::Bound::Unbounded,
					))
					.next()
					.map(|(k, _)| k.clone())
			}
		} else {
			// Looking for last key <= or < search_key
			if include_equal {
				local_keys
					.range(..=search_key.clone())
					.next_back()
					.map(|(k, _)| k.clone())
			} else {
				local_keys
					.range(..search_key.clone())
					.next_back()
					.map(|(k, _)| k.clone())
			}
		};

		// Determine which key to return
		let key = match (best_local, db_key_cleared) {
			(Some(local), false) if !db_key.is_empty() => {
				// Both keys exist, pick the appropriate one based on direction
				if is_forward {
					// Return the smaller key
					if db_key.as_slice() < local.as_slice() {
						db_key
					} else {
						local.into()
					}
				} else {
					// Return the larger key
					if db_key.as_slice() > local.as_slice() {
						db_key
					} else {
						local.into()
					}
				}
			}
			(Some(local), _) => local.into(),
			(None, false) => db_key,
			(None, true) => vec![].into(),
		};

		// Add conflict range on resolved key
		if let IsolationLevel::Serializable = isolation_level {
			self.add_conflict_range(&key, &end_of_key_range(&key), ConflictRangeType::Read);
		}

		Ok(key)
	}

	/// Reads one page of a range and merges this transaction's pending writes into it.
	///
	/// `get_from_db` reads one page of the range it is handed from the database. It runs again, on
	/// the rest of the range, only when pending clears leave nothing of a page that is not the last
	/// one: a page with no rows gives the caller no key to continue from, so it must not be returned
	/// while the range has more rows.
	pub async fn get_range<'a, F, Fut>(
		&self,
		opt: &RangeOption<'a>,
		isolation_level: IsolationLevel,
		get_from_db: F,
	) -> Result<Values>
	where
		F: Fn(RangeOption<'a>) -> Fut,
		Fut: std::future::Future<Output = Result<Values>>,
	{
		if let IsolationLevel::Serializable = isolation_level {
			self.add_conflict_range(opt.begin.key(), opt.end.key(), ConflictRangeType::Read);
		}

		let mut opt = opt.clone();

		loop {
			// Get database results
			let db_values = get_from_db(opt.clone()).await?;

			// If there are no local operations, just return the database results
			if self.operations().is_empty() {
				return Ok(db_values);
			}

			// A page that reports more rows stopped short of the end of the range, so the database
			// has only been read up to the last key of the page.
			let page_end = if db_values.more() {
				db_values.iter().last().map(|kv| kv.key().to_vec())
			} else {
				None
			};

			let merged = self.merge_range_page(&opt, db_values, page_end.as_deref());

			match page_end {
				Some(page_end) if merged.is_empty() => match opt.next_range_after(&page_end, 0) {
					Some(next) => opt = next,
					None => return Ok(merged),
				},
				Some(_) | None => return Ok(merged),
			}
		}
	}

	/// Applies pending writes to one database page of a range read.
	///
	/// `page_end` is the last key of a database page that stopped short of the end of the range.
	/// Pending writes past it belong to a later page. Merging them here would return them ahead of
	/// database rows that sort before them, and the next page would continue after them and skip
	/// those rows.
	fn merge_range_page(
		&self,
		opt: &RangeOption<'_>,
		db_values: Values,
		page_end: Option<&[u8]>,
	) -> Values {
		let in_page = |key: &[u8]| {
			let before_page_end = match page_end {
				Some(page_end) => {
					if opt.reverse {
						key >= page_end
					} else {
						key <= page_end
					}
				}
				None => true,
			};

			before_page_end && range_contains(opt, key)
		};

		// Start with database results in a map
		let mut result_map = BTreeMap::new();
		for kv in db_values.into_iter() {
			let (key, value) = kv.into_parts();
			result_map.insert(key, value);
		}

		// Apply local operations
		for op in &*self.operations() {
			match op {
				Operation::SetValue { key, value } => {
					if in_page(key) {
						result_map.insert(key.clone(), value.clone());
					}
				}
				Operation::Clear { key } => {
					result_map.remove(key);
				}
				Operation::ClearRange {
					begin: clear_begin,
					end: clear_end,
				} => {
					// Remove all keys in the cleared range
					let keys_to_remove: Vec<_> = result_map
						.range(clear_begin.clone()..clear_end.clone())
						.map(|(k, _)| k.clone())
						.collect();
					for key in keys_to_remove {
						result_map.remove(&key);
					}
				}
				Operation::AtomicOp {
					key,
					param,
					op_type,
				} => {
					if in_page(key) {
						// Get current value for this key (from result_map or empty if not exists)
						let current_value = result_map.get(key);
						let current_slice = current_value.map(|v| &**v);

						// Apply the atomic operation
						let new_value = apply_atomic_op(current_slice, param, *op_type);

						if let Some(new_value) = new_value {
							result_map.insert(key.clone(), new_value);
						} else {
							result_map.remove(key);
						}
					}
				}
			}
		}

		// Build result respecting the scan direction and the limit. The merged map is ordered
		// ascending, so a reverse scan has to drain it back to front: otherwise the merge silently
		// flips a reverse scan to ascending, and a limit takes the lowest keys instead of the
		// highest. Reads with no local operations never reach this path, so the direction only ever
		// went wrong once the transaction held a pending write.
		let limit = opt.limit.unwrap_or(usize::MAX);
		// Rows are left unread either past the end of the database page or past the limit.
		let more = page_end.is_some() || result_map.len() > limit;
		let keyvalues = if opt.reverse {
			result_map
				.into_iter()
				.rev()
				.take(limit)
				.map(|(key, value)| KeyValue::new(key, value))
				.collect::<Vec<_>>()
		} else {
			result_map
				.into_iter()
				.take(limit)
				.map(|(key, value)| KeyValue::new(key, value))
				.collect::<Vec<_>>()
		};

		Values::with_more(keyvalues, more)
	}

	pub fn clear_all(&self) {
		self.operations.lock().unwrap().clear();
		self.conflict_ranges.lock().unwrap().clear();
	}

	pub fn add_conflict_range(&self, begin: &[u8], end: &[u8], conflict_type: ConflictRangeType) {
		self.conflict_ranges
			.lock()
			.unwrap()
			.push((begin.to_vec(), end.to_vec(), conflict_type));
	}
}

/// Whether `key` is inside the range `opt` selects, reading its key selectors the way the drivers
/// do. `first_greater_than` excludes its key as a begin bound and includes it as an end bound, and
/// every other selector is an inclusive begin and an exclusive end.
///
/// A paged read continues a forward scan from `first_greater_than(last_key)`, so a pending write to
/// `last_key` has to be excluded here or it is returned once per page.
fn range_contains(opt: &RangeOption<'_>, key: &[u8]) -> bool {
	let after_begin = match (opt.begin.or_equal(), opt.begin.offset()) {
		(true, 1) => key > opt.begin.key(),
		_ => key >= opt.begin.key(),
	};
	let before_end = match (opt.end.or_equal(), opt.end.offset()) {
		(true, 1) => key <= opt.end.key(),
		_ => key < opt.end.key(),
	};

	after_begin && before_end
}
