// Copyright 2018 foundationdb-rs developers, https://github.com/Clikengo/foundationdb-rs/graphs/contributors
// Copyright 2013-2018 Apple, Inc and the FoundationDB project authors.
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Implementations of the FDBTransaction C API
//!
//! <https://apple.github.io/foundationdb/api-c.html#transaction>

use std::{
	borrow::Cow,
	ops::{Range, RangeInclusive},
};

use super::{key_selector::KeySelector, options};
use crate::{tuple::Subspace, value::Values};

// Byte budgets for one page of a range read. They are the ones FoundationDB's client applies per
// batch, taken from `validate_and_update_parameters` in its C binding, so a range read costs the
// same number of pages on every driver.
const SMALL_PAGE_BYTES: usize = 256;
const MEDIUM_PAGE_BYTES: usize = 1_000;
const LARGE_PAGE_BYTES: usize = 4_096;
const SERIAL_PAGE_BYTES: usize = 120_000;

/// Budget of each successive page in `StreamingMode::Iterator`. It starts small so a caller that
/// stops early reads little, and grows so a caller that keeps going is not held to small pages.
const ITERATOR_PAGE_BYTES: [usize; 10] = [
	4_096, 6_144, 9_216, 13_824, 20_736, 31_104, 46_656, 69_984, 80_000, 120_000,
];

/// `RangeOption` represents a query parameters for range scan query.
#[derive(Debug, Clone)]
pub struct RangeOption<'a> {
	/// The beginning of the range.
	pub begin: KeySelector<'a>,
	/// The end of the range.
	pub end: KeySelector<'a>,
	/// If non-zero, indicates the maximum number of key-value pairs to return.
	pub limit: Option<usize>,
	/// If non-zero, indicates a (soft) cap on the combined number of bytes of keys and values to
	/// return in one page. See [`RangeOption::page_target_bytes`].
	pub target_bytes: usize,
	/// One of the options::StreamingMode values indicating how the caller would like the data in
	/// the range returned.
	pub mode: options::StreamingMode,
	/// If true, key-value pairs will be returned in reverse lexicographical order beginning at
	/// the end of the range.
	pub reverse: bool,
	#[doc(hidden)]
	pub __non_exhaustive: std::marker::PhantomData<()>,
}

impl RangeOption<'_> {
	/// Reverses the range direction.
	pub fn rev(mut self) -> Self {
		self.reverse = !self.reverse;
		self
	}

	/// The range left to read after the page `kvs`, or `None` once the range or the limit is used up.
	pub fn next_range(self, kvs: &Values) -> Option<Self> {
		if !kvs.more() {
			return None;
		}

		let last = kvs.iter().last()?;

		self.next_range_after(last.key(), kvs.len())
	}

	/// The range left to read after a page that ended at `last_key` and returned `returned` rows, or
	/// `None` once the limit is used up.
	pub(crate) fn next_range_after(mut self, last_key: &[u8], returned: usize) -> Option<Self> {
		if let Some(limit) = self.limit.as_mut() {
			*limit = limit.saturating_sub(returned);
			if *limit == 0 {
				return None;
			}
		}

		if self.reverse {
			self.end.make_first_greater_or_equal(last_key);
		} else {
			self.begin.make_first_greater_than(last_key);
		}
		Some(self)
	}

	/// Soft cap on the key plus value bytes one page of this range may hold, or `None` when the page
	/// is bounded by `limit` alone.
	///
	/// `iteration` is the 1-based number of the page being read. A page always holds at least one
	/// row and stops after the row that reaches the cap, so a single row larger than the cap still
	/// makes progress. `target_bytes` lowers the cap a mode implies but never raises it.
	///
	/// FoundationDB rejects `StreamingMode::Exact` without a row limit. Here it falls back to the
	/// largest budget instead, so the page stays bounded either way.
	pub fn page_target_bytes(&self, iteration: usize) -> Option<usize> {
		let mode_bytes = match self.mode {
			options::StreamingMode::Exact => match self.limit {
				Some(_) => None,
				None => Some(SERIAL_PAGE_BYTES),
			},
			options::StreamingMode::Small => Some(SMALL_PAGE_BYTES),
			options::StreamingMode::Medium => Some(MEDIUM_PAGE_BYTES),
			options::StreamingMode::Large => Some(LARGE_PAGE_BYTES),
			options::StreamingMode::WantAll | options::StreamingMode::Serial => {
				Some(SERIAL_PAGE_BYTES)
			}
			options::StreamingMode::Iterator => {
				let idx = iteration.clamp(1, ITERATOR_PAGE_BYTES.len()) - 1;
				Some(ITERATOR_PAGE_BYTES[idx])
			}
		};

		match (self.target_bytes, mode_bytes) {
			(0, mode_bytes) => mode_bytes,
			(target_bytes, Some(mode_bytes)) => Some(target_bytes.min(mode_bytes)),
			(target_bytes, None) => Some(target_bytes),
		}
	}
}

impl Default for RangeOption<'_> {
	fn default() -> Self {
		Self {
			begin: KeySelector::first_greater_or_equal([].as_ref()),
			end: KeySelector::first_greater_or_equal([].as_ref()),
			limit: None,
			target_bytes: 0,
			mode: options::StreamingMode::Iterator,
			reverse: false,
			__non_exhaustive: std::marker::PhantomData,
		}
	}
}

impl<'a> From<(KeySelector<'a>, KeySelector<'a>)> for RangeOption<'a> {
	fn from((begin, end): (KeySelector<'a>, KeySelector<'a>)) -> Self {
		Self {
			begin,
			end,
			..Self::default()
		}
	}
}
impl From<(Vec<u8>, Vec<u8>)> for RangeOption<'static> {
	fn from((begin, end): (Vec<u8>, Vec<u8>)) -> Self {
		Self {
			begin: KeySelector::first_greater_or_equal(begin),
			end: KeySelector::first_greater_or_equal(end),
			..Self::default()
		}
	}
}
impl<'a> From<(&'a [u8], &'a [u8])> for RangeOption<'a> {
	fn from((begin, end): (&'a [u8], &'a [u8])) -> Self {
		Self {
			begin: KeySelector::first_greater_or_equal(begin),
			end: KeySelector::first_greater_or_equal(end),
			..Self::default()
		}
	}
}
impl<'a> From<std::ops::Range<KeySelector<'a>>> for RangeOption<'a> {
	fn from(range: Range<KeySelector<'a>>) -> Self {
		RangeOption::from((range.start, range.end))
	}
}

impl<'a> From<std::ops::Range<&'a [u8]>> for RangeOption<'a> {
	fn from(range: Range<&'a [u8]>) -> Self {
		RangeOption::from((range.start, range.end))
	}
}

impl From<std::ops::Range<std::vec::Vec<u8>>> for RangeOption<'static> {
	fn from(range: Range<Vec<u8>>) -> Self {
		RangeOption::from((range.start, range.end))
	}
}

impl<'a> From<std::ops::RangeInclusive<&'a [u8]>> for RangeOption<'a> {
	fn from(range: RangeInclusive<&'a [u8]>) -> Self {
		let (start, end) = range.into_inner();
		(KeySelector::first_greater_or_equal(start)..KeySelector::first_greater_than(end)).into()
	}
}

impl From<std::ops::RangeInclusive<std::vec::Vec<u8>>> for RangeOption<'static> {
	fn from(range: RangeInclusive<Vec<u8>>) -> Self {
		let (start, end) = range.into_inner();
		(KeySelector::first_greater_or_equal(start)..KeySelector::first_greater_than(end)).into()
	}
}

impl<'a> From<&'a Subspace> for RangeOption<'static> {
	fn from(subspace: &Subspace) -> Self {
		let (begin, end) = subspace.range();

		Self {
			begin: KeySelector::first_greater_or_equal(Cow::Owned(begin)),

			end: KeySelector::first_greater_or_equal(Cow::Owned(end)),

			..Self::default()
		}
	}
}
