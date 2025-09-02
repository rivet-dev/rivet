use std::result::Result::Ok;

use anyhow::*;
use rivet_util::Id;
use udb_util::prelude::*;

#[derive(Debug)]
pub struct LeaseKey {
	pub workflow_id: Id,
}

impl LeaseKey {
	pub fn new(workflow_id: Id) -> Self {
		LeaseKey { workflow_id }
	}

	pub fn subspace() -> LeaseSubspaceKey {
		LeaseSubspaceKey::new()
	}
}

impl FormalKey for LeaseKey {
	/// Workflow name, worker instance id.
	type Value = (String, Id);

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for LeaseKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, LEASE, self.workflow_id);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LeaseKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id)) = <(usize, usize, Id)>::unpack(input, tuple_depth)?;
		let v = LeaseKey { workflow_id };

		Ok((input, v))
	}
}

pub struct LeaseSubspaceKey {}

impl LeaseSubspaceKey {
	pub fn new() -> Self {
		LeaseSubspaceKey {}
	}
}

impl TuplePack for LeaseSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, LEASE);
		t.pack(w, tuple_depth)
	}
}

pub struct TagKey {
	workflow_id: Id,
	pub k: String,
	pub v: String,
}

impl TagKey {
	pub fn new(workflow_id: Id, k: String, v: String) -> Self {
		TagKey { workflow_id, k, v }
	}

	pub fn subspace(workflow_id: Id) -> TagSubspaceKey {
		TagSubspaceKey::new(workflow_id)
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
		let t = (WORKFLOW, DATA, self.workflow_id, TAG, &self.k, &self.v);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TagKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data, k, v)) =
			<(usize, usize, Id, usize, String, String)>::unpack(input, tuple_depth)?;
		if data != TAG {
			return Err(PackError::Message("expected TAG data".into()));
		}

		let v = TagKey { workflow_id, k, v };

		Ok((input, v))
	}
}

pub struct TagSubspaceKey {
	workflow_id: Id,
}

impl TagSubspaceKey {
	pub fn new(workflow_id: Id) -> Self {
		TagSubspaceKey { workflow_id }
	}
}

impl TuplePack for TagSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, TAG);
		t.pack(w, tuple_depth)
	}
}

pub struct InputKey {
	workflow_id: Id,
}

impl InputKey {
	pub fn new(workflow_id: Id) -> Self {
		InputKey { workflow_id }
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

impl FormalChunkedKey for InputKey {
	type ChunkKey = InputChunkKey;
	type Value = Box<serde_json::value::RawValue>;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		InputChunkKey {
			workflow_id: self.workflow_id,
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

impl TuplePack for InputKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, INPUT);
		t.pack(w, tuple_depth)
	}
}

pub struct InputChunkKey {
	workflow_id: Id,
	chunk: usize,
}

impl TuplePack for InputChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, INPUT, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for InputChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data, chunk)) =
			<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
		if data != INPUT {
			return Err(PackError::Message("expected INPUT data".into()));
		}

		let v = InputChunkKey { workflow_id, chunk };

		Ok((input, v))
	}
}

pub struct OutputKey {
	workflow_id: Id,
}

impl OutputKey {
	pub fn new(workflow_id: Id) -> Self {
		OutputKey { workflow_id }
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

impl FormalChunkedKey for OutputKey {
	type ChunkKey = OutputChunkKey;
	type Value = Box<serde_json::value::RawValue>;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		OutputChunkKey {
			workflow_id: self.workflow_id,
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

impl TuplePack for OutputKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, OUTPUT);
		t.pack(w, tuple_depth)
	}
}

pub struct OutputChunkKey {
	workflow_id: Id,
	chunk: usize,
}

impl TuplePack for OutputChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, OUTPUT, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for OutputChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data, chunk)) =
			<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
		if data != OUTPUT {
			return Err(PackError::Message("expected OUTPUT data".into()));
		}

		let v = OutputChunkKey { workflow_id, chunk };

		Ok((input, v))
	}
}

pub struct StateKey {
	workflow_id: Id,
}

impl StateKey {
	pub fn new(workflow_id: Id) -> Self {
		StateKey { workflow_id }
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

impl FormalChunkedKey for StateKey {
	type ChunkKey = StateChunkKey;
	type Value = Box<serde_json::value::RawValue>;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		StateChunkKey {
			workflow_id: self.workflow_id,
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

impl TuplePack for StateKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, STATE);
		t.pack(w, tuple_depth)
	}
}

pub struct StateChunkKey {
	workflow_id: Id,
	chunk: usize,
}

impl TuplePack for StateChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, STATE, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for StateChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data, chunk)) =
			<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
		if data != STATE {
			return Err(PackError::Message("expected STATE data".into()));
		}

		let v = StateChunkKey { workflow_id, chunk };

		Ok((input, v))
	}
}

pub struct WakeSignalKey {
	workflow_id: Id,
	pub signal_name: String,
}

impl WakeSignalKey {
	pub fn new(workflow_id: Id, signal_name: String) -> Self {
		WakeSignalKey {
			workflow_id,
			signal_name,
		}
	}

	pub fn subspace(workflow_id: Id) -> WakeSignalSubspaceKey {
		WakeSignalSubspaceKey::new(workflow_id)
	}
}

impl FormalKey for WakeSignalKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for WakeSignalKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			WORKFLOW,
			DATA,
			self.workflow_id,
			WAKE_SIGNAL,
			&self.signal_name,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WakeSignalKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data, signal_name)) =
			<(usize, usize, Id, usize, String)>::unpack(input, tuple_depth)?;
		if data != WAKE_SIGNAL {
			return Err(PackError::Message("expected WAKE_SIGNAL data".into()));
		}

		let v = WakeSignalKey {
			workflow_id,
			signal_name,
		};

		Ok((input, v))
	}
}

pub struct WakeSignalSubspaceKey {
	workflow_id: Id,
}

impl WakeSignalSubspaceKey {
	pub fn new(workflow_id: Id) -> Self {
		WakeSignalSubspaceKey { workflow_id }
	}
}

impl TuplePack for WakeSignalSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, WAKE_SIGNAL);
		t.pack(w, tuple_depth)
	}
}

pub struct WakeDeadlineKey {
	workflow_id: Id,
}

impl WakeDeadlineKey {
	pub fn new(workflow_id: Id) -> Self {
		WakeDeadlineKey { workflow_id }
	}
}

impl FormalKey for WakeDeadlineKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for WakeDeadlineKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, WAKE_DEADLINE);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WakeDeadlineKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != WAKE_DEADLINE {
			return Err(PackError::Message("expected WAKE_DEADLINE data".into()));
		}
		let v = WakeDeadlineKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NameKey {
	workflow_id: Id,
}

impl NameKey {
	pub fn new(workflow_id: Id) -> Self {
		NameKey { workflow_id }
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
		let t = (WORKFLOW, DATA, self.workflow_id, NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != NAME {
			return Err(PackError::Message("expected NAME data".into()));
		}

		let v = NameKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	workflow_id: Id,
}

impl CreateTsKey {
	pub fn new(workflow_id: Id) -> Self {
		CreateTsKey { workflow_id }
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
		let t = (WORKFLOW, DATA, self.workflow_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != CREATE_TS {
			return Err(PackError::Message("expected CREATE_TS data".into()));
		}

		let v = CreateTsKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RayIdKey {
	workflow_id: Id,
}

impl RayIdKey {
	pub fn new(workflow_id: Id) -> Self {
		RayIdKey { workflow_id }
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
		let t = (WORKFLOW, DATA, self.workflow_id, RAY_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RayIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != RAY_ID {
			return Err(PackError::Message("expected RAY_ID data".into()));
		}

		let v = RayIdKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ErrorKey {
	workflow_id: Id,
}

impl ErrorKey {
	pub fn new(workflow_id: Id) -> Self {
		ErrorKey { workflow_id }
	}
}

impl FormalKey for ErrorKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for ErrorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, ERROR);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ErrorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != ERROR {
			return Err(PackError::Message("expected ERROR data".into()));
		}

		let v = ErrorKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WakeSubWorkflowKey {
	workflow_id: Id,
}

impl WakeSubWorkflowKey {
	pub fn new(workflow_id: Id) -> Self {
		WakeSubWorkflowKey { workflow_id }
	}
}

impl FormalKey for WakeSubWorkflowKey {
	/// Sub workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for WakeSubWorkflowKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, WAKE_SUB_WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WakeSubWorkflowKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != WAKE_SUB_WORKFLOW_ID {
			return Err(PackError::Message(
				"expected WAKE_SUB_WORKFLOW_ID data".into(),
			));
		}

		let v = WakeSubWorkflowKey { workflow_id };

		Ok((input, v))
	}
}

pub struct PendingSignalKey {
	pub workflow_id: Id,
	pub signal_name: String,
	/// For ordering.
	pub ts: i64,
	pub signal_id: Id,
}

impl PendingSignalKey {
	pub fn new(workflow_id: Id, signal_name: String, signal_id: Id) -> Self {
		PendingSignalKey {
			workflow_id,
			signal_name,
			ts: rivet_util::timestamp::now(),
			signal_id,
		}
	}

	pub fn subspace(workflow_id: Id, signal_name: String) -> PendingSignalSubspaceKey {
		PendingSignalSubspaceKey::new(workflow_id, signal_name)
	}
}

impl FormalKey for PendingSignalKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for PendingSignalKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			WORKFLOW,
			SIGNAL,
			self.workflow_id,
			PENDING,
			&self.signal_name,
			self.ts,
			self.signal_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for PendingSignalKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _, signal_name, ts, signal_id)) =
			<(usize, usize, Id, usize, String, i64, Id)>::unpack(input, tuple_depth)?;
		let v = PendingSignalKey {
			workflow_id,
			signal_name,
			ts,
			signal_id,
		};

		Ok((input, v))
	}
}

pub struct PendingSignalSubspaceKey {
	workflow_id: Id,
	signal_name: String,
}

impl PendingSignalSubspaceKey {
	pub fn new(workflow_id: Id, signal_name: String) -> Self {
		PendingSignalSubspaceKey {
			workflow_id,
			signal_name,
		}
	}
}

impl TuplePack for PendingSignalSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			WORKFLOW,
			SIGNAL,
			self.workflow_id,
			PENDING,
			&self.signal_name,
		);
		t.pack(w, tuple_depth)
	}
}

pub struct ByNameAndTagKey {
	workflow_name: String,
	k: String,
	v: String,
	pub workflow_id: Id,
}

impl ByNameAndTagKey {
	pub fn new(workflow_name: String, k: String, v: String, workflow_id: Id) -> Self {
		ByNameAndTagKey {
			workflow_name,
			k,
			v,
			workflow_id,
		}
	}

	pub fn subspace(workflow_name: String, k: String, v: String) -> ByNameAndTagSubspaceKey {
		ByNameAndTagSubspaceKey::new(workflow_name, k, v)
	}

	pub fn null(workflow_name: String, workflow_id: Id) -> Self {
		ByNameAndTagKey {
			workflow_name,
			k: String::new(),
			v: String::new(),
			workflow_id,
		}
	}

	pub fn null_subspace(workflow_name: String) -> ByNameAndTagSubspaceKey {
		ByNameAndTagSubspaceKey::null(workflow_name)
	}
}

impl FormalKey for ByNameAndTagKey {
	// Rest of the tags.
	type Value = Vec<(String, String)>;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		serde_json::from_slice(raw).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		serde_json::to_vec(&value).map_err(Into::into)
	}
}

impl TuplePack for ByNameAndTagKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			WORKFLOW,
			BY_NAME_AND_TAG,
			&self.workflow_name,
			&self.k,
			&self.v,
			self.workflow_id,
		);

		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ByNameAndTagKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_name, k, v, workflow_id)) =
			<(usize, usize, String, String, String, Id)>::unpack(input, tuple_depth)?;
		let v = ByNameAndTagKey {
			workflow_name,
			k,
			v,
			workflow_id,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ByNameAndTagSubspaceKey {
	workflow_name: String,
	k: String,
	v: String,
}

impl ByNameAndTagSubspaceKey {
	pub fn new(workflow_name: String, k: String, v: String) -> Self {
		ByNameAndTagSubspaceKey {
			workflow_name,
			k,
			v,
		}
	}

	pub fn null(workflow_name: String) -> Self {
		ByNameAndTagSubspaceKey {
			workflow_name,
			k: String::new(),
			v: String::new(),
		}
	}
}

impl TuplePack for ByNameAndTagSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			WORKFLOW,
			BY_NAME_AND_TAG,
			&self.workflow_name,
			&self.k,
			&self.v,
		);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct HasWakeConditionKey {
	pub workflow_id: Id,
}

impl HasWakeConditionKey {
	pub fn new(workflow_id: Id) -> Self {
		HasWakeConditionKey { workflow_id }
	}
}

impl FormalKey for HasWakeConditionKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for HasWakeConditionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, HAS_WAKE_CONDITION);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for HasWakeConditionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != HAS_WAKE_CONDITION {
			return Err(PackError::Message(
				"expected HAS_WAKE_CONDITION data".into(),
			));
		}

		let v = HasWakeConditionKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WorkerInstanceIdKey {
	pub workflow_id: Id,
}

impl WorkerInstanceIdKey {
	pub fn new(workflow_id: Id) -> Self {
		WorkerInstanceIdKey { workflow_id }
	}
}

impl FormalKey for WorkerInstanceIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for WorkerInstanceIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (WORKFLOW, DATA, self.workflow_id, WORKER_INSTANCE_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkerInstanceIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != WORKER_INSTANCE_ID {
			return Err(PackError::Message(
				"expected WORKER_INSTANCE_ID data".into(),
			));
		}

		let v = WorkerInstanceIdKey { workflow_id };

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
		let t = (WORKFLOW, DATA);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct SilenceTsKey {
	workflow_id: Id,
}

impl SilenceTsKey {
	pub fn new(workflow_id: Id) -> Self {
		SilenceTsKey { workflow_id }
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
		let t = (WORKFLOW, DATA, self.workflow_id, SILENCE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SilenceTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, data)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		if data != SILENCE_TS {
			return Err(PackError::Message("expected SILENCE_TS data".into()));
		}

		let v = SilenceTsKey { workflow_id };

		Ok((input, v))
	}
}
