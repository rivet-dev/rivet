use anyhow::*;
use foundationdb as fdb;
use prost::Message;
use serde::Serialize;

use crate::{key::Key, metadata::Metadata};

#[derive(Default)]
pub(crate) struct EntryBuilder {
	metadata: Option<Metadata>,
	value: Vec<u8>,
	next_idx: usize,
}

impl EntryBuilder {
	pub(crate) fn add_sub_key(&mut self, sub_key: SubKey) -> Result<()> {
		match sub_key {
			SubKey::Metadata(value) => {
				// We ignore setting the metadata again because it means the same key was given twice in the
				// input keys for `ActorKv::get`. We don't perform automatic deduplication.
				if self.metadata.is_none() {
					self.metadata = Some(Metadata::decode(value.value())?);
				}
			}
			SubKey::Chunk(idx, value) => {
				// We don't perform deduplication on the input keys for `ActorKv::get` so we might have
				// duplicate data chunks. This idx check ignores chunks that were already passed and ensures
				// contiguity.
				if idx == self.next_idx {
					self.value.extend(value.value());
					self.next_idx = idx + 1;
				}
			}
		}

		Ok(())
	}

	pub(crate) fn build(self, key: &Key) -> Result<Entry> {
		ensure!(!self.value.is_empty(), "empty value at key {key:?}");

		Ok(Entry {
			metadata: self
				.metadata
				.with_context(|| format!("no metadata for key {key:?}"))?,
			value: self.value,
		})
	}
}

/// Represents a Rivet KV value.
#[derive(Serialize)]
pub struct Entry {
	pub metadata: Metadata,
	pub value: Vec<u8>,
}

/// Represents FDB keys within a Rivet KV key.
pub(crate) enum SubKey {
	Metadata(fdb::future::FdbValue),
	Chunk(usize, fdb::future::FdbValue),
}
