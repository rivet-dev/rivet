use std::result::Result::Ok;

use anyhow::*;
use udb_util::prelude::*;

use rivet_runner_protocol as rp;

use crate::key::KeyWrapper;

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

// Parses key in first position, ignores the rest
pub struct EntryBaseKey {
	pub key: KeyWrapper,
}

impl<'de> TupleUnpack<'de> for EntryBaseKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, key) = <KeyWrapper>::unpack(input, tuple_depth)?;
		let v = EntryBaseKey { key };

		Ok((&input[0..0], v))
	}
}

pub struct EntryValueChunkKey {
	key: KeyWrapper,
	pub chunk: usize,
}

impl EntryValueChunkKey {
	pub fn new(key: KeyWrapper, chunk: usize) -> Self {
		EntryValueChunkKey { key, chunk }
	}
}

impl TuplePack for EntryValueChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (&self.key, DATA, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for EntryValueChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (key, data, chunk)) = <(KeyWrapper, usize, usize)>::unpack(input, tuple_depth)?;
		if data != DATA {
			return Err(PackError::Message("expected DATA data".into()));
		}

		let v = EntryValueChunkKey { key, chunk };

		Ok((input, v))
	}
}

pub struct EntryMetadataKey {
	pub key: KeyWrapper,
}

impl EntryMetadataKey {
	pub fn new(key: KeyWrapper) -> Self {
		EntryMetadataKey { key }
	}
}

impl FormalKey for EntryMetadataKey {
	type Value = rp::KvMetadata;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_bare::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_bare::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for EntryMetadataKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (&self.key, METADATA);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for EntryMetadataKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (key, data)) = <(KeyWrapper, usize)>::unpack(input, tuple_depth)?;
		if data != METADATA {
			return Err(PackError::Message("expected METADATA data".into()));
		}

		let v = EntryMetadataKey { key };

		Ok((input, v))
	}
}
