use std::result::Result::Ok;

use anyhow::*;
use rivet_util::Id;
use udb_util::prelude::*;

pub struct BodyKey {
	signal_id: Id,
}

impl BodyKey {
	pub fn new(signal_id: Id) -> Self {
		BodyKey { signal_id }
	}

	pub fn split_ref(&self, value: &serde_json::value::RawValue) -> Result<Vec<Vec<u8>>> {
		Ok(value
			.get()
			.as_bytes()
			.chunks(udb_util::CHUNK_SIZE)
			.map(|x| x.to_vec())
			.collect())
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
		let t = (SIGNAL, DATA, self.signal_id, BODY);
		t.pack(w, tuple_depth)
	}
}

pub struct BodyChunkKey {
	signal_id: Id,
	chunk: usize,
}

impl TuplePack for BodyChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (SIGNAL, DATA, self.signal_id, BODY, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for BodyChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data, chunk)) =
			<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
		if data != BODY {
			return Err(PackError::Message("expected BODY data".into()));
		}

		let v = BodyChunkKey { signal_id, chunk };

		Ok((input, v))
	}
}

pub struct AckTsKey {
	signal_id: Id,
}

impl AckTsKey {
	pub fn new(signal_id: Id) -> Self {
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
		let t = (SIGNAL, DATA, self.signal_id, ACK_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AckTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != ACK_TS {
			return Err(PackError::Message("expected ACK_TS data".into()));
		}

		let v = AckTsKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	signal_id: Id,
}

impl CreateTsKey {
	pub fn new(signal_id: Id) -> Self {
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
		let t = (SIGNAL, DATA, self.signal_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != CREATE_TS {
			return Err(PackError::Message("expected CREATE_TS data".into()));
		}

		let v = CreateTsKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RayIdKey {
	signal_id: Id,
}

impl RayIdKey {
	pub fn new(signal_id: Id) -> Self {
		RayIdKey { signal_id }
	}
}

impl FormalKey for RayIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
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
		let t = (SIGNAL, DATA, self.signal_id, RAY_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RayIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != RAY_ID {
			return Err(PackError::Message("expected RAY_ID data".into()));
		}

		let v = RayIdKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NameKey {
	signal_id: Id,
}

impl NameKey {
	pub fn new(signal_id: Id) -> Self {
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
		let t = (SIGNAL, DATA, self.signal_id, NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != NAME {
			return Err(PackError::Message("expected NAME data".into()));
		}

		let v = NameKey { signal_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	signal_id: Id,
}

impl WorkflowIdKey {
	pub fn new(signal_id: Id) -> Self {
		WorkflowIdKey { signal_id }
	}
}

impl FormalKey for WorkflowIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
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
		let t = (SIGNAL, DATA, self.signal_id, WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != WORKFLOW_ID {
			return Err(PackError::Message("expected WORKFLOW_ID data".into()));
		}

		let v = WorkflowIdKey { signal_id };

		Ok((input, v))
	}
}

pub struct DataSubspaceKey {}

impl DataSubspaceKey {
	pub fn new() -> Self {
		DataSubspaceKey {}
	}
}

impl TuplePack for DataSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (SIGNAL, DATA);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct SilenceTsKey {
	signal_id: Id,
}

impl SilenceTsKey {
	pub fn new(signal_id: Id) -> Self {
		SilenceTsKey { signal_id }
	}
}

impl FormalKey for SilenceTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for SilenceTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (SIGNAL, DATA, self.signal_id, SILENCE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SilenceTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, signal_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != SILENCE_TS {
			return Err(PackError::Message("expected SILENCE_TS data".into()));
		}

		let v = SilenceTsKey { signal_id };

		Ok((input, v))
	}
}
