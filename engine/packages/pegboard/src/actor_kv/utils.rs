use anyhow::{Result, ensure};
use rivet_envoy_protocol as ep;

use super::{
	MAX_KEY_SIZE, MAX_KEYS, MAX_PUT_PAYLOAD_SIZE, MAX_STORAGE_SIZE, MAX_VALUE_SIZE,
	keys::actor_kv::KeyWrapper,
};
use crate::errors;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryValidationErrorKind {
	LengthMismatch,
	TooManyEntries,
	PayloadTooLarge,
	StorageQuotaExceeded,
	KeyTooLarge,
	ValueTooLarge,
}

#[derive(Debug)]
pub struct EntryValidationError {
	kind: EntryValidationErrorKind,
	remaining: Option<usize>,
	payload_size: Option<usize>,
}

impl EntryValidationError {
	pub fn kind(&self) -> EntryValidationErrorKind {
		self.kind
	}

	pub fn into_anyhow(self) -> anyhow::Error {
		match self.kind {
			EntryValidationErrorKind::LengthMismatch => {
				anyhow::Error::msg("Keys list length != values list length")
			}
			EntryValidationErrorKind::TooManyEntries => {
				anyhow::Error::msg("A maximum of 128 key-value entries is allowed")
			}
			EntryValidationErrorKind::PayloadTooLarge => {
				anyhow::Error::msg("total payload is too large (max 976 KiB)")
			}
			EntryValidationErrorKind::StorageQuotaExceeded => {
				errors::Actor::KvStorageQuotaExceeded {
					remaining: self.remaining.unwrap_or_default(),
					payload_size: self.payload_size.unwrap_or_default(),
				}
				.build()
				.into()
			}
			EntryValidationErrorKind::KeyTooLarge => {
				anyhow::Error::msg("key is too long (max 2048 bytes)")
			}
			EntryValidationErrorKind::ValueTooLarge => anyhow::Error::msg(format!(
				"value is too large (max {} KiB)",
				MAX_VALUE_SIZE / 1024
			)),
		}
	}
}

pub fn validate_list_query(query: &ep::KvListQuery) -> Result<()> {
	match query {
		ep::KvListQuery::KvListAllQuery => {}
		ep::KvListQuery::KvListRangeQuery(range) => {
			validate_range(&range.start, &range.end)?;
		}
		ep::KvListQuery::KvListPrefixQuery(prefix) => {
			ensure!(
				KeyWrapper::tuple_len(&prefix.key) <= MAX_KEY_SIZE,
				"prefix key is too long (max 2048 bytes)"
			);
		}
	}

	Ok(())
}

pub fn validate_range(start: &ep::KvKey, end: &ep::KvKey) -> Result<()> {
	ensure!(
		KeyWrapper::tuple_len(start) <= MAX_KEY_SIZE,
		"start key is too long (max 2048 bytes)"
	);
	ensure!(
		KeyWrapper::tuple_len(end) <= MAX_KEY_SIZE,
		"end key is too long (max 2048 bytes)"
	);

	Ok(())
}

pub fn validate_keys(keys: &[ep::KvKey]) -> Result<()> {
	ensure!(keys.len() <= MAX_KEYS, "a maximum of 128 keys is allowed");

	for key in keys {
		ensure!(
			KeyWrapper::tuple_len(key) <= MAX_KEY_SIZE,
			"key is too long (max 2048 bytes)"
		);
	}

	Ok(())
}

pub fn validate_entries_with_details(
	keys: &[ep::KvKey],
	values: &[ep::KvValue],
	total_size: usize,
) -> std::result::Result<(), EntryValidationError> {
	if keys.len() != values.len() {
		return Err(EntryValidationError {
			kind: EntryValidationErrorKind::LengthMismatch,
			remaining: None,
			payload_size: None,
		});
	}
	if keys.len() > MAX_KEYS || values.len() > MAX_KEYS {
		return Err(EntryValidationError {
			kind: EntryValidationErrorKind::TooManyEntries,
			remaining: None,
			payload_size: None,
		});
	}
	let payload_size = keys.iter().fold(0, |acc, k| acc + KeyWrapper::tuple_len(k))
		+ values.iter().fold(0, |acc, v| acc + v.len());
	if payload_size > MAX_PUT_PAYLOAD_SIZE {
		return Err(EntryValidationError {
			kind: EntryValidationErrorKind::PayloadTooLarge,
			remaining: None,
			payload_size: Some(payload_size),
		});
	}

	let storage_remaining = MAX_STORAGE_SIZE.saturating_sub(total_size);
	if payload_size > storage_remaining {
		return Err(EntryValidationError {
			kind: EntryValidationErrorKind::StorageQuotaExceeded,
			remaining: Some(storage_remaining),
			payload_size: Some(payload_size),
		});
	}

	for key in keys {
		if KeyWrapper::tuple_len(key) > MAX_KEY_SIZE {
			return Err(EntryValidationError {
				kind: EntryValidationErrorKind::KeyTooLarge,
				remaining: None,
				payload_size: None,
			});
		}
	}

	for value in values {
		if value.len() > MAX_VALUE_SIZE {
			return Err(EntryValidationError {
				kind: EntryValidationErrorKind::ValueTooLarge,
				remaining: None,
				payload_size: None,
			});
		}
	}

	Ok(())
}
