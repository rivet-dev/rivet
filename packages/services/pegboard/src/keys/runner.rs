use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;
use versioned_data_util::OwnedVersionedData;

#[derive(Debug)]
pub struct CreateTsKey {
	runner_id: Id,
}

impl CreateTsKey {
	pub fn new(runner_id: Id) -> Self {
		CreateTsKey { runner_id }
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
		let t = (RUNNER, DATA, self.runner_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RemainingSlotsKey {
	runner_id: Id,
}

impl RemainingSlotsKey {
	pub fn new(runner_id: Id) -> Self {
		RemainingSlotsKey { runner_id }
	}
}

impl FormalKey for RemainingSlotsKey {
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u32::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for RemainingSlotsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, REMAINING_SLOTS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RemainingSlotsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = RemainingSlotsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastPingTsKey {
	runner_id: Id,
}

impl LastPingTsKey {
	pub fn new(runner_id: Id) -> Self {
		LastPingTsKey { runner_id }
	}
}

impl FormalKey for LastPingTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for LastPingTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, LAST_PING_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct TotalSlotsKey {
	runner_id: Id,
}

impl TotalSlotsKey {
	pub fn new(runner_id: Id) -> Self {
		TotalSlotsKey { runner_id }
	}
}

impl FormalKey for TotalSlotsKey {
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u32::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for TotalSlotsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, TOTAL_SLOTS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for TotalSlotsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = TotalSlotsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActorKey {
	runner_id: Id,
	pub actor_id: Id,
}

impl ActorKey {
	pub fn new(runner_id: Id, actor_id: Id) -> Self {
		ActorKey {
			runner_id,
			actor_id,
		}
	}

	pub fn subspace(runner_id: Id) -> ActorSubspaceKey {
		ActorSubspaceKey::new(runner_id)
	}

	pub fn entire_subspace() -> ActorSubspaceKey {
		ActorSubspaceKey::entire()
	}
}

impl FormalKey for ActorKey {
	/// Generation.
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		if raw.is_empty() {
			Ok(0)
		} else {
			Ok(u32::from_be_bytes(raw.try_into()?))
		}
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for ActorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, ACTOR, self.runner_id, self.actor_id);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, actor_id)) =
			<(usize, usize, Id, Id)>::unpack(input, tuple_depth)?;
		let v = ActorKey {
			runner_id,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct ActorSubspaceKey {
	runner_id: Option<Id>,
}

impl ActorSubspaceKey {
	fn new(runner_id: Id) -> Self {
		ActorSubspaceKey {
			runner_id: Some(runner_id),
		}
	}

	fn entire() -> Self {
		ActorSubspaceKey { runner_id: None }
	}
}

impl TuplePack for ActorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (RUNNER, ACTOR);
		offset += t.pack(w, tuple_depth)?;

		if let Some(runner_id) = &self.runner_id {
			offset += runner_id.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	runner_id: Id,
}

impl WorkflowIdKey {
	pub fn new(runner_id: Id) -> Self {
		WorkflowIdKey { runner_id }
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
		let t = (RUNNER, DATA, self.runner_id, WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = WorkflowIdKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NamespaceIdKey {
	runner_id: Id,
}

impl NamespaceIdKey {
	pub fn new(runner_id: Id) -> Self {
		NamespaceIdKey { runner_id }
	}
}

impl FormalKey for NamespaceIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for NamespaceIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, NAMESPACE_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NamespaceIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = NamespaceIdKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NameKey {
	runner_id: Id,
}

impl NameKey {
	pub fn new(runner_id: Id) -> Self {
		NameKey { runner_id }
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
		let t = (RUNNER, DATA, self.runner_id, NAME);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = NameKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct KeyKey {
	runner_id: Id,
}

impl KeyKey {
	pub fn new(runner_id: Id) -> Self {
		KeyKey { runner_id }
	}
}

impl FormalKey for KeyKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for KeyKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, KEY);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for KeyKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = KeyKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct VersionKey {
	runner_id: Id,
}

impl VersionKey {
	pub fn new(runner_id: Id) -> Self {
		VersionKey { runner_id }
	}
}

impl FormalKey for VersionKey {
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u32::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for VersionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, VERSION);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for VersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = VersionKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct AddressKey {
	pub runner_id: Id,
	pub name: String,
}

impl AddressKey {
	pub fn new(runner_id: Id, name: String) -> Self {
		AddressKey { runner_id, name }
	}

	pub fn subspace(runner_id: Id) -> AddressSubspaceKey {
		AddressSubspaceKey::new(runner_id)
	}
}

impl FormalKey for AddressKey {
	type Value = <rivet_key_data::versioned::AddressKeyData as OwnedVersionedData>::Latest;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		rivet_key_data::versioned::AddressKeyData::deserialize_with_embedded_version(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_key_data::versioned::AddressKeyData::latest(value)
			.serialize_with_embedded_version(rivet_key_data::PEGBOARD_RUNNER_ADDRESS_VERSION)
	}
}

impl TuplePack for AddressKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, ADDRESS, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AddressKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _, name)) =
			<(usize, usize, Id, usize, String)>::unpack(input, tuple_depth)?;

		let v = AddressKey { runner_id, name };

		Ok((input, v))
	}
}

pub struct AddressSubspaceKey {
	runner_id: Id,
}

impl AddressSubspaceKey {
	pub fn new(runner_id: Id) -> Self {
		AddressSubspaceKey { runner_id }
	}
}

impl TuplePack for AddressSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, ADDRESS);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct StopTsKey {
	runner_id: Id,
}

impl StopTsKey {
	pub fn new(runner_id: Id) -> Self {
		StopTsKey { runner_id }
	}
}

impl FormalKey for StopTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for StopTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, STOP_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for StopTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = StopTsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct DrainTsKey {
	runner_id: Id,
}

impl DrainTsKey {
	pub fn new(runner_id: Id) -> Self {
		DrainTsKey { runner_id }
	}
}

impl FormalKey for DrainTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for DrainTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, DRAIN_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for DrainTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = DrainTsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastRttKey {
	runner_id: Id,
}

impl LastRttKey {
	pub fn new(runner_id: Id) -> Self {
		LastRttKey { runner_id }
	}
}

impl FormalKey for LastRttKey {
	// Milliseconds.
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u32::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for LastRttKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, LAST_RTT);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastRttKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = LastRttKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ConnectedTsKey {
	runner_id: Id,
}

impl ConnectedTsKey {
	pub fn new(runner_id: Id) -> Self {
		ConnectedTsKey { runner_id }
	}
}

impl FormalKey for ConnectedTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for ConnectedTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, CONNECTED_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ConnectedTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = ConnectedTsKey { runner_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ExpiredTsKey {
	runner_id: Id,
}

impl ExpiredTsKey {
	pub fn new(runner_id: Id) -> Self {
		ExpiredTsKey { runner_id }
	}
}

impl FormalKey for ExpiredTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for ExpiredTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, EXPIRED_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ExpiredTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, _)) =
			<(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = ExpiredTsKey { runner_id };

		Ok((input, v))
	}
}

pub struct MetadataKey {
	runner_id: Id,
}

impl MetadataKey {
	pub fn new(runner_id: Id) -> Self {
		MetadataKey { runner_id }
	}
}

impl FormalChunkedKey for MetadataKey {
	type ChunkKey = MetadataChunkKey;
	type Value = rivet_key_data::converted::MetadataKeyData;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		MetadataChunkKey {
			runner_id: self.runner_id,
			chunk,
		}
	}

	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value> {
		rivet_key_data::versioned::MetadataKeyData::deserialize_with_embedded_version(
			&chunks
				.iter()
				.map(|x| x.value().iter().map(|x| *x))
				.flatten()
				.collect::<Vec<_>>(),
		)?
		.try_into()
	}

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
		Ok(
			rivet_key_data::versioned::MetadataKeyData::latest(value.try_into()?)
				.serialize_with_embedded_version(rivet_key_data::PEGBOARD_RUNNER_METADATA_VERSION)?
				.chunks(udb_util::CHUNK_SIZE)
				.map(|x| x.to_vec())
				.collect(),
		)
	}
}

impl TuplePack for MetadataKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, METADATA);
		t.pack(w, tuple_depth)
	}
}

pub struct MetadataChunkKey {
	runner_id: Id,
	chunk: usize,
}

impl TuplePack for MetadataChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (RUNNER, DATA, self.runner_id, METADATA, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for MetadataChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, runner_id, data, chunk)) =
			<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
		if data != METADATA {
			return Err(PackError::Message("expected METADATA data".into()));
		}

		let v = MetadataChunkKey { runner_id, chunk };

		Ok((input, v))
	}
}
