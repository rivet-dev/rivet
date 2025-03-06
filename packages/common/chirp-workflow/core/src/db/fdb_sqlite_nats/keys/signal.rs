use std::{borrow::Cow, result::Result::Ok};

// TODO: Use concrete error types
use anyhow::*;
use foundationdb::{
	future::FdbValue,
	tuple::{PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset},
};
use uuid::Uuid;

use super::{FormalChunkedKey, FormalKey};

pub struct BodyKey {
	signal_id: Uuid,
}

impl BodyKey {
	pub fn new(signal_id: Uuid) -> Self {
		BodyKey { signal_id }
	}

	pub fn split_ref(&self, value: &serde_json::value::RawValue) -> Result<Vec<Vec<u8>>> {
		// TODO: Chunk
		Ok(vec![value.get().as_bytes().to_vec()])
	}
}

impl FormalChunkedKey for BodyKey {
	type ChunkKey = BodyChunkKey;
	type Value = Box<serde_json::value::RawValue>;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		BodyChunkKey {
			signal_id: self.signal_id,
			chunk,
		}
	}

	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value> {
		serde_json::value::RawValue::from_string(String::from_utf8(
			chunks
				.iter()
				.map(|x| x.value().iter().map(|x| *x))
				.flatten()
				.collect(),
		)?)
		.map_err(Into::into)
	}

	// fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
	// 	self.split_ref(value.as_ref())
	// }
}

impl TuplePack for BodyKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "body");
		t.pack(w, tuple_depth)
	}
}

pub struct BodyChunkKey {
	signal_id: Uuid,
	chunk: usize,
}

impl TuplePack for BodyChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "body", self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for BodyChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, _, chunk)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, usize)>::unpack(input, tuple_depth)?;
		let v = BodyChunkKey { signal_id, chunk };

		Ok((input, v))
	}
}

pub struct AckKey {
	signal_id: Uuid,
}

impl AckKey {
	pub fn new(signal_id: Uuid) -> Self {
		AckKey { signal_id }
	}
}

impl FormalKey for AckKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for AckKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "ack");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AckKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = AckKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	signal_id: Uuid,
}

impl CreateTsKey {
	pub fn new(signal_id: Uuid) -> Self {
		CreateTsKey { signal_id }
	}
}

impl FormalKey for CreateTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for CreateTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "create_ts");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { signal_id };

		Ok((input, v))
	}
}

pub struct TaggedPendingKey {
	pub signal_name: String,
	/// For ordering.
	pub ts: i64,
	pub signal_id: Uuid,
}

impl TaggedPendingKey {
	pub fn new(signal_name: String, signal_id: Uuid) -> Self {
		TaggedPendingKey {
			signal_name,
			ts: rivet_util::timestamp::now(),
			signal_id,
		}
	}

	pub fn subspace(signal_name: String) -> TaggedPendingSubspaceKey {
		TaggedPendingSubspaceKey::new(signal_name)
	}
}

impl FormalKey for TaggedPendingKey {
	/// Signal tags.
	type Value = Vec<(String, String)>;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for TaggedPendingKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			"tagged_signal",
			"pending",
			&self.signal_name,
			self.ts,
			self.signal_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TaggedPendingKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_name, ts, signal_id)) =
			<(Cow<str>, Cow<str>, String, i64, Uuid)>::unpack(input, tuple_depth)?;
		let v = TaggedPendingKey {
			signal_name,
			ts,
			signal_id,
		};

		Ok((input, v))
	}
}

pub struct TaggedPendingSubspaceKey {
	signal_name: String,
}

impl TaggedPendingSubspaceKey {
	fn new(signal_name: String) -> Self {
		TaggedPendingSubspaceKey { signal_name }
	}
}

impl TuplePack for TaggedPendingSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("tagged_signal", "pending", &self.signal_name);
		t.pack(w, tuple_depth)
	}
}

pub struct TagKey {
	signal_id: Uuid,
	pub k: String,
	pub v: String,
}

impl TagKey {
	pub fn new(signal_id: Uuid, k: String, v: String) -> Self {
		TagKey { signal_id, k, v }
	}

	// pub fn subspace(signal_id: Uuid) -> TagSubspaceKey {
	// 	TagSubspaceKey::new(signal_id)
	// }
}

impl FormalKey for TagKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for TagKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "tag", &self.k, &self.v);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TagKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, _, k, v)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, String, String)>::unpack(input, tuple_depth)?;
		let v = TagKey { signal_id, k, v };

		Ok((input, v))
	}
}

// pub struct TagSubspaceKey {
// 	signal_id: Uuid,
// }

// impl TagSubspaceKey {
// 	pub fn new(signal_id: Uuid) -> Self {
// 		TagSubspaceKey { signal_id }
// 	}
// }

// impl TuplePack for TagSubspaceKey {
// 	fn pack<W: std::io::Write>(
// 		&self,
// 		w: &mut W,
// 		tuple_depth: TupleDepth,
// 	) -> std::io::Result<VersionstampOffset> {
// 		let t = ("signal", "data", self.signal_id, "tag");
// 		t.pack(w, tuple_depth)
// 	}
// }

#[derive(Debug)]
pub struct RayIdKey {
	signal_id: Uuid,
}

impl RayIdKey {
	pub fn new(signal_id: Uuid) -> Self {
		RayIdKey { signal_id }
	}
}

impl FormalKey for RayIdKey {
	type Value = Uuid;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Uuid::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for RayIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "ray_id");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RayIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = RayIdKey { signal_id };

		Ok((input, v))
	}
}
