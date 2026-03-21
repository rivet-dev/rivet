use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use universaldb::prelude::*;
use vbare::OwnedVersionedData;

#[derive(Debug)]
pub struct CreateTsKey {
	namespace_id: Id,
	envoy_key: String,
}

impl CreateTsKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		CreateTsKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			CREATE_TS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ExpiredTsKey {
	namespace_id: Id,
	envoy_key: String,
}

impl ExpiredTsKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		ExpiredTsKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			EXPIRED_TS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ExpiredTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = ExpiredTsKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct SlotsKey {
	namespace_id: Id,
	envoy_key: String,
}

impl SlotsKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		SlotsKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl FormalKey for SlotsKey {
	/// Count.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		// NOTE: Atomic ops use little endian
		Ok(i64::from_le_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		// NOTE: Atomic ops use little endian
		Ok(value.to_le_bytes().to_vec())
	}
}

impl TuplePack for SlotsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			SLOTS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SlotsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = SlotsKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct PoolNameKey {
	namespace_id: Id,
	envoy_key: String,
}

impl PoolNameKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		PoolNameKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl FormalKey for PoolNameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for PoolNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			POOL_NAME,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for PoolNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;

		let v = PoolNameKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct VersionKey {
	namespace_id: Id,
	envoy_key: String,
}

impl VersionKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		VersionKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			VERSION,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for VersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;

		let v = VersionKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct StopTsKey {
	namespace_id: Id,
	envoy_key: String,
}

impl StopTsKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		StopTsKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			STOP_TS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for StopTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = StopTsKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ProtocolVersionKey {
	namespace_id: Id,
	envoy_key: String,
}

impl ProtocolVersionKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		ProtocolVersionKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl FormalKey for ProtocolVersionKey {
	type Value = u16;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u16::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for ProtocolVersionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			PROTOCOL_VERSION,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ProtocolVersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = ProtocolVersionKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastRttKey {
	namespace_id: Id,
	envoy_key: String,
}

impl LastRttKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		LastRttKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			LAST_RTT,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastRttKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = LastRttKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ConnectedTsKey {
	namespace_id: Id,
	envoy_key: String,
}

impl ConnectedTsKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		ConnectedTsKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			CONNECTED_TS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ConnectedTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = ConnectedTsKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct LastPingTsKey {
	namespace_id: Id,
	envoy_key: String,
}

impl LastPingTsKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		LastPingTsKey {
			namespace_id,
			envoy_key,
		}
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			LAST_PING_TS,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LastPingTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = LastPingTsKey {
			namespace_id,
			envoy_key,
		};

		Ok((input, v))
	}
}

pub struct MetadataKey {
	namespace_id: Id,
	envoy_key: String,
}

impl MetadataKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		MetadataKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl FormalChunkedKey for MetadataKey {
	type ChunkKey = MetadataChunkKey;
	type Value = rivet_data::converted::MetadataKeyData;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		MetadataChunkKey {
			namespace_id: self.namespace_id,
			envoy_key: self.envoy_key.clone(),
			chunk,
		}
	}

	fn combine(&self, chunks: Vec<Value>) -> Result<Self::Value> {
		rivet_data::versioned::MetadataKeyData::deserialize_with_embedded_version(
			&chunks
				.iter()
				.map(|x| x.value().iter().map(|x| *x))
				.flatten()
				.collect::<Vec<_>>(),
		)
		.context("failed to combine `MetadataKey`")?
		.try_into()
	}

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
		Ok(
			rivet_data::versioned::MetadataKeyData::wrap_latest(value.try_into()?)
				.serialize_with_embedded_version(rivet_data::PEGBOARD_RUNNER_METADATA_VERSION)?
				.chunks(universaldb::utils::CHUNK_SIZE)
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
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			METADATA,
		);
		t.pack(w, tuple_depth)
	}
}

pub struct MetadataChunkKey {
	namespace_id: Id,
	envoy_key: String,
	chunk: usize,
}

impl TuplePack for MetadataChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			METADATA,
			self.chunk,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for MetadataChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, data, chunk)) =
			<(usize, usize, usize, Id, String, usize, usize)>::unpack(input, tuple_depth)?;
		if data != METADATA {
			return Err(PackError::Message("expected METADATA data".into()));
		}

		let v = MetadataChunkKey {
			namespace_id,
			envoy_key,
			chunk,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActorLastCommandIdxKey {
	namespace_id: Id,
	envoy_key: String,
	actor_id: Id,
	generation: u32,
}

impl ActorLastCommandIdxKey {
	pub fn new(namespace_id: Id, envoy_key: String, actor_id: Id, generation: u32) -> Self {
		ActorLastCommandIdxKey {
			namespace_id,
			envoy_key,
			actor_id,
			generation,
		}
	}
}

impl FormalKey for ActorLastCommandIdxKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for ActorLastCommandIdxKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			ACTOR,
			LAST_COMMAND_IDX,
			self.actor_id,
			self.generation,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorLastCommandIdxKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _, _, actor_id, generation)) =
			<(usize, usize, usize, Id, String, usize, usize, Id, u32)>::unpack(input, tuple_depth)?;
		let v = ActorLastCommandIdxKey {
			namespace_id,
			envoy_key,
			actor_id,
			generation,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActorCommandKey {
	pub namespace_id: Id,
	pub envoy_key: String,
	pub actor_id: Id,
	pub generation: u32,
	pub index: i64,
}

impl ActorCommandKey {
	pub fn new(
		namespace_id: Id,
		envoy_key: String,
		actor_id: Id,
		generation: u32,
		index: i64,
	) -> Self {
		ActorCommandKey {
			namespace_id,
			envoy_key,
			actor_id,
			generation,
			index,
		}
	}

	pub fn subspace(namespace_id: Id, envoy_key: String) -> ActorCommandSubspaceKey {
		ActorCommandSubspaceKey::new(namespace_id, envoy_key)
	}

	pub fn subspace_with_actor(
		namespace_id: Id,
		envoy_key: String,
		actor_id: Id,
		generation: u32,
	) -> ActorCommandSubspaceKey {
		ActorCommandSubspaceKey::new_with_actor(namespace_id, envoy_key, actor_id, generation)
	}

	pub fn subspace_with_index(
		namespace_id: Id,
		envoy_key: String,
		actor_id: Id,
		generation: u32,
		index: i64,
	) -> ActorCommandSubspaceKey {
		ActorCommandSubspaceKey::new_with_index(
			namespace_id,
			envoy_key,
			actor_id,
			generation,
			index,
		)
	}
}

impl FormalKey for ActorCommandKey {
	type Value = rivet_envoy_protocol::ActorCommandKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		rivet_envoy_protocol::versioned::ActorCommandKeyData::deserialize_with_embedded_version(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_envoy_protocol::versioned::ActorCommandKeyData::wrap_latest(value)
			.serialize_with_embedded_version(rivet_envoy_protocol::PROTOCOL_VERSION)
	}
}

impl TuplePack for ActorCommandKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			ACTOR,
			COMMAND,
			self.actor_id,
			self.generation,
			self.index,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorCommandKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _, _, actor_id, generation, index)) =
			<(usize, usize, usize, Id, String, usize, usize, Id, u32, i64)>::unpack(
				input,
				tuple_depth,
			)?;
		let v = ActorCommandKey {
			namespace_id,
			envoy_key,
			actor_id,
			generation,
			index,
		};

		Ok((input, v))
	}
}

pub struct ActorCommandSubspaceKey {
	namespace_id: Id,
	envoy_key: String,
	actor_id: Option<Id>,
	generation: Option<u32>,
	index: Option<i64>,
}

impl ActorCommandSubspaceKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		ActorCommandSubspaceKey {
			namespace_id,
			envoy_key,
			actor_id: None,
			generation: None,
			index: None,
		}
	}

	pub fn new_with_actor(
		namespace_id: Id,
		envoy_key: String,
		actor_id: Id,
		generation: u32,
	) -> Self {
		ActorCommandSubspaceKey {
			namespace_id,
			envoy_key,
			actor_id: Some(actor_id),
			generation: Some(generation),
			index: None,
		}
	}

	pub fn new_with_index(
		namespace_id: Id,
		envoy_key: String,
		actor_id: Id,
		generation: u32,
		index: i64,
	) -> Self {
		ActorCommandSubspaceKey {
			namespace_id,
			envoy_key,
			actor_id: Some(actor_id),
			generation: Some(generation),
			index: Some(index),
		}
	}
}

impl TuplePack for ActorCommandSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			NAMESPACE,
			ENVOY,
			DATA,
			self.namespace_id,
			&self.envoy_key,
			ACTOR,
			COMMAND,
		);
		offset += t.pack(w, tuple_depth)?;

		if let Some(actor_id) = &self.actor_id {
			offset += actor_id.pack(w, tuple_depth)?;

			if let Some(v) = &self.generation {
				offset += v.pack(w, tuple_depth)?;

				if let Some(index) = &self.index {
					offset += index.pack(w, tuple_depth)?;
				}
			}
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct ActorKey {
	namespace_id: Id,
	envoy_key: String,
	pub actor_id: Id,
}

impl ActorKey {
	pub fn new(namespace_id: Id, envoy_key: String, actor_id: Id) -> Self {
		ActorKey {
			namespace_id,
			envoy_key,
			actor_id,
		}
	}

	pub fn subspace(namespace_id: Id, envoy_key: String) -> ActorSubspaceKey {
		ActorSubspaceKey::new(namespace_id, envoy_key)
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
		let t = (
			NAMESPACE,
			ENVOY,
			ACTOR,
			self.namespace_id,
			&self.envoy_key,
			self.actor_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, actor_id)) =
			<(usize, usize, usize, Id, String, Id)>::unpack(input, tuple_depth)?;
		let v = ActorKey {
			namespace_id,
			envoy_key,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct ActorSubspaceKey {
	namespace_id: Id,
	envoy_key: String,
}

impl ActorSubspaceKey {
	fn new(namespace_id: Id, envoy_key: String) -> Self {
		ActorSubspaceKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl TuplePack for ActorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (NAMESPACE, ENVOY, ACTOR, self.namespace_id, &self.envoy_key);
		offset += t.pack(w, tuple_depth)?;

		Ok(offset)
	}
}
