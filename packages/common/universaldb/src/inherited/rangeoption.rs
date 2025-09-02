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

use super::{keyselector::*, options};
use crate::{future::*, tuple::Subspace};

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
	/// return for each item.
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

	pub fn next_range(mut self, kvs: &FdbValues) -> Option<Self> {
		if !kvs.more() {
			return None;
		}

		let last = kvs.iter().last()?;
		let last_key = last.key();

		if let Some(limit) = self.limit.as_mut() {
			*limit = limit.saturating_sub(kvs.len());
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
