use std::{collections::HashMap, result::Result::Ok};

use anyhow::*;
use deno_core::JsBuffer;
use foundationdb as fdb;
use futures_util::{FutureExt, TryStreamExt};

use crate::{
	key::Key, MAX_KEYS, MAX_KEY_SIZE, MAX_PUT_PAYLOAD_SIZE, MAX_STORAGE_SIZE, MAX_VALUE_SIZE,
};

pub trait TransactionExt {
	/// Owned version of `Transaction.get_ranges` (self is owned).
	fn get_ranges_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValues>>
	       + Send
	       + Sync
	       + Unpin
	       + 'a;

	/// Owned version of `Transaction.get_ranges_keyvalues` (self is owned).
	fn get_ranges_keyvalues_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValue>> + Unpin + 'a;
}

impl TransactionExt for fdb::RetryableTransaction {
	fn get_ranges_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValues>>
	       + Send
	       + Sync
	       + Unpin
	       + 'a {
		futures_util::stream::unfold((1, Some(opt)), move |(iteration, maybe_opt)| {
			if let Some(opt) = maybe_opt {
				futures_util::future::Either::Left(
					self.get_range(&opt, iteration as usize, snapshot)
						.map(move |maybe_values| {
							let next_opt = match &maybe_values {
								Ok(values) => opt.next_range(values),
								Err(..) => None,
							};
							Some((maybe_values, (iteration + 1, next_opt)))
						}),
				)
			} else {
				futures_util::future::Either::Right(std::future::ready(None))
			}
		})
	}

	fn get_ranges_keyvalues_owned<'a>(
		self,
		opt: fdb::RangeOption<'a>,
		snapshot: bool,
	) -> impl futures_util::Stream<Item = fdb::FdbResult<fdb::future::FdbValue>> + Unpin + 'a {
		self.get_ranges_owned(opt, snapshot)
			.map_ok(|values| futures_util::stream::iter(values.into_iter().map(Ok)))
			.try_flatten()
	}
}

pub fn now() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
		.as_millis()
		.try_into()
		.expect("now doesn't fit in i64")
}

pub fn validate_keys(keys: &[Key]) -> Result<()> {
	ensure!(keys.len() <= MAX_KEYS, "a maximum of 128 keys is allowed");

	for key in keys {
		ensure!(
			key.len() <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
	}

	Ok(())
}

pub fn validate_entries(entries: &HashMap<Key, JsBuffer>, total_size: usize) -> Result<()> {
	ensure!(
		entries.len() <= MAX_KEYS,
		"A maximum of 128 key-value entries is allowed"
	);
	let payload_size = entries
		.iter()
		.fold(0, |acc, (k, v)| acc + k.len() + v.len());
	ensure!(
		payload_size <= MAX_PUT_PAYLOAD_SIZE,
		"total payload is too large (max 976 KiB)"
	);

	let storage_remaining = MAX_STORAGE_SIZE.saturating_sub(total_size);
	ensure!(
		payload_size <= storage_remaining,
		"not enough space left in storage ({storage_remaining} bytes remaining, current payload is {payload_size} bytes)"
	);

	for (key, value) in entries {
		ensure!(
			key.len() <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
		ensure!(
			value.len() <= MAX_VALUE_SIZE,
			"value is too large (max 128 KiB)"
		);
	}

	Ok(())
}
