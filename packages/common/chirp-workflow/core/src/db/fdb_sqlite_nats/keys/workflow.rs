use std::result::Result::Ok;

use anyhow::*;
use fdb_util::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub struct LeaseKey {
	pub workflow_id: Uuid,
}

impl LeaseKey {
	pub fn new(workflow_id: Uuid) -> Self {
		LeaseKey { workflow_id }
	}

	pub fn subspace() -> LeaseSubspaceKey {
		LeaseSubspaceKey::new()
	}
}

impl FormalKey for LeaseKey {
	/// Workflow name, worker instance id.
	type Value = (String, Uuid);

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
		let t = ("workflow", "lease", self.workflow_id);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LeaseKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id)) =
			<(Cow<str>, Cow<str>, Uuid)>::unpack(input, tuple_depth)?;
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
		let t = ("workflow", "lease");
		t.pack(w, tuple_depth)
	}
}

pub struct TagKey {
	workflow_id: Uuid,
	pub k: String,
	pub v: String,
}

impl TagKey {
	pub fn new(workflow_id: Uuid, k: String, v: String) -> Self {
		TagKey { workflow_id, k, v }
	}

	pub fn subspace(workflow_id: Uuid) -> TagSubspaceKey {
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
		let t = (
			"workflow",
			"data",
			self.workflow_id,
			"tag",
			&self.k,
			&self.v,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TagKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _, k, v)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, String, String)>::unpack(input, tuple_depth)?;
		let v = TagKey { workflow_id, k, v };

		Ok((input, v))
	}
}

pub struct TagSubspaceKey {
	workflow_id: Uuid,
}

impl TagSubspaceKey {
	pub fn new(workflow_id: Uuid) -> Self {
		TagSubspaceKey { workflow_id }
	}
}

impl TuplePack for TagSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("workflow", "data", self.workflow_id, "tag");
		t.pack(w, tuple_depth)
	}
}

pub struct InputKey {
	workflow_id: Uuid,
}

impl InputKey {
	pub fn new(workflow_id: Uuid) -> Self {
		InputKey { workflow_id }
	}

	pub fn split_ref(&self, value: &serde_json::value::RawValue) -> Result<Vec<Vec<u8>>> {
		// TODO: Chunk
		Ok(vec![value.get().as_bytes().to_vec()])
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
		let t = ("workflow", "data", self.workflow_id, "input");
		t.pack(w, tuple_depth)
	}
}

pub struct InputChunkKey {
	workflow_id: Uuid,
	chunk: usize,
}

impl TuplePack for InputChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("workflow", "data", self.workflow_id, "input", self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for InputChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, workflow_id, _, chunk)) =
			<(Cow<str>, Uuid, Cow<str>, usize)>::unpack(input, tuple_depth)?;
		let v = InputChunkKey { workflow_id, chunk };

		Ok((input, v))
	}
}

pub struct OutputKey {
	workflow_id: Uuid,
}

impl OutputKey {
	pub fn new(workflow_id: Uuid) -> Self {
		OutputKey { workflow_id }
	}

	pub fn split_ref(&self, value: &serde_json::value::RawValue) -> Result<Vec<Vec<u8>>> {
		// TODO: Chunk
		Ok(vec![value.get().as_bytes().to_vec()])
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
		let t = ("workflow", "data", self.workflow_id, "output");
		t.pack(w, tuple_depth)
	}
}

pub struct OutputChunkKey {
	workflow_id: Uuid,
	chunk: usize,
}

impl TuplePack for OutputChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("workflow", "data", self.workflow_id, "output", self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for OutputChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, workflow_id, _, chunk)) =
			<(Cow<str>, Uuid, Cow<str>, usize)>::unpack(input, tuple_depth)?;
		let v = OutputChunkKey { workflow_id, chunk };

		Ok((input, v))
	}
}

pub struct WakeSignalKey {
	workflow_id: Uuid,
	pub signal_name: String,
}

impl WakeSignalKey {
	pub fn new(workflow_id: Uuid, signal_name: String) -> Self {
		WakeSignalKey {
			workflow_id,
			signal_name,
		}
	}

	pub fn subspace(workflow_id: Uuid) -> WakeSignalSubspaceKey {
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
			"workflow",
			"data",
			self.workflow_id,
			"wake_signal",
			&self.signal_name,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WakeSignalKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _, signal_name)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, String)>::unpack(input, tuple_depth)?;
		let v = WakeSignalKey {
			workflow_id,
			signal_name,
		};

		Ok((input, v))
	}
}

pub struct WakeSignalSubspaceKey {
	workflow_id: Uuid,
}

impl WakeSignalSubspaceKey {
	pub fn new(workflow_id: Uuid) -> Self {
		WakeSignalSubspaceKey { workflow_id }
	}
}

impl TuplePack for WakeSignalSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("workflow", "data", self.workflow_id, "wake_signal");
		t.pack(w, tuple_depth)
	}
}

pub struct WakeDeadlineKey {
	workflow_id: Uuid,
}

impl WakeDeadlineKey {
	pub fn new(workflow_id: Uuid) -> Self {
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
		let t = ("workflow", "data", self.workflow_id, "wake_deadline");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WakeDeadlineKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = WakeDeadlineKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NameKey {
	workflow_id: Uuid,
}

impl NameKey {
	pub fn new(workflow_id: Uuid) -> Self {
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
		let t = ("workflow", "data", self.workflow_id, "name");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = NameKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	workflow_id: Uuid,
}

impl CreateTsKey {
	pub fn new(workflow_id: Uuid) -> Self {
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
		let t = ("workflow", "data", self.workflow_id, "create_ts");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RayIdKey {
	workflow_id: Uuid,
}

impl RayIdKey {
	pub fn new(workflow_id: Uuid) -> Self {
		RayIdKey { workflow_id }
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
		let t = ("workflow", "data", self.workflow_id, "ray_id");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RayIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = RayIdKey { workflow_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ErrorKey {
	workflow_id: Uuid,
}

impl ErrorKey {
	pub fn new(workflow_id: Uuid) -> Self {
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
		let t = ("workflow", "data", self.workflow_id, "error");
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ErrorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, workflow_id, _)) =
			<(Cow<str>, Cow<str>, Uuid, Cow<str>)>::unpack(input, tuple_depth)?;
		let v = ErrorKey { workflow_id };

		Ok((input, v))
	}
}

pub struct PendingSignalKey {
	pub workflow_id: Uuid,
	pub signal_name: String,
	/// For ordering.
	pub ts: i64,
	pub signal_id: Uuid,
}

impl PendingSignalKey {
	pub fn new(workflow_id: Uuid, signal_name: String, signal_id: Uuid) -> Self {
		PendingSignalKey {
			workflow_id,
			signal_name,
			ts: rivet_util::timestamp::now(),
			signal_id,
		}
	}

	pub fn subspace(workflow_id: Uuid, signal_name: String) -> PendingSignalSubspaceKey {
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
			"workflow",
			"signal",
			self.workflow_id,
			"pending",
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
			<(Cow<str>, Cow<str>, Uuid, Cow<str>, String, i64, Uuid)>::unpack(input, tuple_depth)?;
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
	workflow_id: Uuid,
	signal_name: String,
}

impl PendingSignalSubspaceKey {
	fn new(workflow_id: Uuid, signal_name: String) -> Self {
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
			"workflow",
			"signal",
			self.workflow_id,
			"pending",
			&self.signal_name,
		);
		t.pack(w, tuple_depth)
	}
}

pub struct ByNameAndTagKey {
	workflow_name: String,
	k: String,
	v: String,
	pub workflow_id: Uuid,
}

impl ByNameAndTagKey {
	pub fn new(workflow_name: String, k: String, v: String, workflow_id: Uuid) -> Self {
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

	pub fn null(workflow_name: String, workflow_id: Uuid) -> Self {
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
			"workflow",
			"by_name_and_tag",
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
			<(Cow<str>, Cow<str>, String, String, String, Uuid)>::unpack(input, tuple_depth)?;
		let v = ByNameAndTagKey {
			workflow_name,
			k,
			v,
			workflow_id,
		};

		Ok((input, v))
	}
}

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
			"workflow",
			"by_name_and_tag",
			&self.workflow_name,
			&self.k,
			&self.v,
		);
		t.pack(w, tuple_depth)
	}
}
