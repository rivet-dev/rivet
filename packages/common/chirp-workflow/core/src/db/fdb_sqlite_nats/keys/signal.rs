use std::result::Result::Ok;

use anyhow::*;
use fdb_util::prelude::*;
use uuid::Uuid;

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

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
		self.split_ref(value.as_ref())
	}
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
		let (input, (_, _, signal_id, data, chunk)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, usize)>::unpack(input, tuple_depth)?;
		if data != "body" {
			return Err(PackError::Message("expected \"body\" data".into()));
		}
		
		let v = BodyChunkKey { signal_id, chunk };

		Ok((input, v))
	}
}

pub struct AckTsKey {
	signal_id: Uuid,
}

impl AckTsKey {
	pub fn new(signal_id: Uuid) -> Self {
		AckTsKey { signal_id }
	}
}

impl FormalKey for AckTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for AckTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "ack_ts");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AckTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		if data != "ack_ts" {
			return Err(PackError::Message("expected \"ack_ts\" data".into()));
		}
		
		let v = AckTsKey { signal_id };

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
		let (input, (_, _, signal_id, data)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		if data != "create_ts" {
			return Err(PackError::Message("expected \"create_ts\" data".into()));
		}
		
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

	pub fn subspace(signal_id: Uuid) -> TagSubspaceKey {
		TagSubspaceKey::new(signal_id)
	}
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
		let (input, (_, _, signal_id, data, k, v)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, String, String)>::unpack(input, tuple_depth)?;
		if data != "tag" {
			return Err(PackError::Message("expected \"tag\" data".into()));
		}

		let v = TagKey { signal_id, k, v };

		Ok((input, v))
	}
}

pub struct TagSubspaceKey {
	signal_id: Uuid,
}

impl TagSubspaceKey {
	pub fn new(signal_id: Uuid) -> Self {
		TagSubspaceKey { signal_id }
	}
}

impl TuplePack for TagSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "tag");
		t.pack(w, tuple_depth)
	}
}

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
		let (input, (_, _, signal_id, data)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		if data != "ray_id" {
			return Err(PackError::Message("expected \"ray_id\" data".into()));
		}

		let v = RayIdKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NameKey {
	signal_id: Uuid,
}

impl NameKey {
	pub fn new(signal_id: Uuid) -> Self {
		NameKey { signal_id }
	}
}

impl FormalKey for NameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for NameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "name");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		if data != "name" {
			return Err(PackError::Message("expected \"name\" data".into()));
		}

		let v = NameKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	signal_id: Uuid,
}

impl WorkflowIdKey {
	pub fn new(signal_id: Uuid) -> Self {
		WorkflowIdKey { signal_id }
	}
}

impl FormalKey for WorkflowIdKey {
	type Value = Uuid;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Uuid::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for WorkflowIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("signal", "data", self.signal_id, "workflow_id");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		if data != "workflow_id" {
			return Err(PackError::Message("expected \"workflow_id\" data".into()));
		}

		let v = WorkflowIdKey { signal_id };

		Ok((input, v))
	}
}
