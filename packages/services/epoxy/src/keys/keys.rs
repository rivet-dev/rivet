use anyhow::*;
use epoxy_protocol::protocol::ReplicaId;
use std::result::Result::Ok;
use udb_util::prelude::*;

#[derive(Debug, Clone)]
pub struct KvValueKey {
	key: Vec<u8>,
}

impl KvValueKey {
	pub fn new(key: Vec<u8>) -> Self {
		Self { key }
	}

	pub fn key(&self) -> &[u8] {
		&self.key
	}
}

impl FormalKey for KvValueKey {
	type Value = Vec<u8>;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(raw.to_vec())
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value)
	}
}

impl TuplePack for KvValueKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (KV, &self.key, COMMITTED_VALUE);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for KvValueKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, key, _)) = <(usize, Vec<u8>, usize)>::unpack(input, tuple_depth)?;

		let v = KvValueKey { key };

		Ok((input, v))
	}
}

#[derive(Debug, Clone)]
pub struct KvOptimisticCacheKey {
	key: Vec<u8>,
}

impl KvOptimisticCacheKey {
	pub fn new(key: Vec<u8>) -> Self {
		Self { key }
	}

	pub fn key(&self) -> &[u8] {
		&self.key
	}
}

impl FormalKey for KvOptimisticCacheKey {
	type Value = Vec<u8>;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(raw.to_vec())
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value)
	}
}

impl TuplePack for KvOptimisticCacheKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (KV, &self.key, OPTIMISTIC_CACHED_VALUE);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for KvOptimisticCacheKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, key, _)) = <(usize, Vec<u8>, usize)>::unpack(input, tuple_depth)?;

		let v = KvOptimisticCacheKey { key };

		Ok((input, v))
	}
}
