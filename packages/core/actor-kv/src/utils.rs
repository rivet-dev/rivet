use std::result::Result::Ok;

use anyhow::*;
use rivet_runner_protocol as rp;

use crate::{
	MAX_KEY_SIZE, MAX_KEYS, MAX_PUT_PAYLOAD_SIZE, MAX_STORAGE_SIZE, MAX_VALUE_SIZE, key::KeyWrapper,
};

pub fn now() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
		.as_millis()
		.try_into()
		.expect("now doesn't fit in i64")
}

pub fn validate_list_query(query: &rp::KvListQuery) -> Result<()> {
	match query {
		rp::KvListQuery::KvListAllQuery => {}
		rp::KvListQuery::KvListRangeQuery(range) => {
			ensure!(
				KeyWrapper::tuple_len(&range.start) <= MAX_KEY_SIZE,
				"start key is too long (max 2048 bytes)"
			);
			ensure!(
				KeyWrapper::tuple_len(&range.end) <= MAX_KEY_SIZE,
				"end key is too long (max 2048 bytes)"
			);
		}
		rp::KvListQuery::KvListPrefixQuery(prefix) => {
			ensure!(
				KeyWrapper::tuple_len(&prefix.key) <= MAX_KEY_SIZE,
				"prefix key is too long (max 2048 bytes)"
			);
		}
	}

	Ok(())
}

pub fn validate_keys(keys: &[rp::KvKey]) -> Result<()> {
	ensure!(keys.len() <= MAX_KEYS, "a maximum of 128 keys is allowed");

	for key in keys {
		ensure!(
			KeyWrapper::tuple_len(key) <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
	}

	Ok(())
}

pub fn validate_entries(
	keys: &[rp::KvKey],
	values: &[rp::KvValue],
	total_size: usize,
) -> Result<()> {
	ensure!(
		keys.len() == values.len(),
		"Keys list length != values list length"
	);
	ensure!(
		keys.len() <= MAX_KEYS,
		"A maximum of 128 key-value entries is allowed"
	);
	ensure!(
		values.len() <= MAX_KEYS,
		"A maximum of 128 key-value entries is allowed"
	);
	let payload_size = keys.iter().fold(0, |acc, k| acc + KeyWrapper::tuple_len(k))
		+ values.iter().fold(0, |acc, v| acc + v.len());
	ensure!(
		payload_size <= MAX_PUT_PAYLOAD_SIZE,
		"total payload is too large (max 976 KiB)"
	);

	let storage_remaining = MAX_STORAGE_SIZE.saturating_sub(total_size);
	ensure!(
		payload_size <= storage_remaining,
		"not enough space left in storage ({storage_remaining} bytes remaining, current payload is {payload_size} bytes)"
	);

	for key in keys {
		ensure!(
			KeyWrapper::tuple_len(key) <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
	}

	for value in values {
		ensure!(
			value.len() <= MAX_VALUE_SIZE,
			"value is too large (max 128 KiB)"
		);
	}

	Ok(())
}
