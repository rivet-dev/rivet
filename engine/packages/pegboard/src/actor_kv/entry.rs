use anyhow::{Result, context, ensure};
use universaldb::prelude::*;

use rivet_runner_protocol::mk2 as rp;

use crate::keys::actor_kv::KeyWrapper;

pub struct EntryBuilder {
	pub key: KeyWrapper,
	metadata: Option<rp::KvMetadata>,
	value: Vec<u8>,
	next_idx: usize,
}

impl EntryBuilder {
	pub fn new(key: KeyWrapper) -> Self {
		EntryBuilder {
			key,
			metadata: None,
			value: Vec::new(),
			next_idx: 0,
		}
	}

	pub fn append_metadata(&mut self, metadata: rp::KvMetadata) {
		// We ignore setting the metadata again because it means the same key was given twice in the
		// input keys for `get`. We don't perform automatic deduplication.
		if self.metadata.is_none() {
			self.metadata = Some(metadata);
		}
	}

	pub fn append_chunk(&mut self, idx: usize, chunk: &[u8]) {
		if idx >= self.next_idx {
			self.value.extend(chunk);
			self.next_idx = idx + 1;
		}
	}

	pub fn build(self) -> Result<(rp::KvKey, rp::KvValue, rp::KvMetadata)> {
		ensure!(!self.value.is_empty(), "empty value at key");

		Ok((
			self.key.0,
			self.value,
			self.metadata.context("no metadata for key")?,
		))
	}
}
