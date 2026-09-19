use std::result::Result::Ok;
use std::time::{Duration, Instant};

use anyhow::*;
use futures_util::TryStreamExt;
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
pub struct VirtualNodesKey {
	namespace_id: Id,
	envoy_key: String,
}

impl VirtualNodesKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		VirtualNodesKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl FormalKey for VirtualNodesKey {
	type Value = u8;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u8::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for VirtualNodesKey {
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
			VIRTUAL_NODES,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for VirtualNodesKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;

		let v = VirtualNodesKey {
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
/// Identifies the connection that currently owns an Envoy registration.
///
/// This additive key is ignored by older binaries. Deployments must drain connections running
/// versions that do not enforce it before relying on the fence for handoff safety.
pub struct EnvoyConnIdKey {
	namespace_id: Id,
	envoy_key: String,
}

impl EnvoyConnIdKey {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		EnvoyConnIdKey {
			namespace_id,
			envoy_key,
		}
	}
}

impl FormalKey for EnvoyConnIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for EnvoyConnIdKey {
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
			ENVOY_CONN_ID,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for EnvoyConnIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _)) =
			<(usize, usize, usize, Id, String, usize)>::unpack(input, tuple_depth)?;
		let v = EnvoyConnIdKey {
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

impl ActorCommandKey {
	fn is_same_command(&self, other: &ActorCommandKey) -> bool {
		self.namespace_id == other.namespace_id
			&& self.envoy_key == other.envoy_key
			&& self.actor_id == other.actor_id
			&& self.generation == other.generation
			&& self.index == other.index
	}
}

impl FormalChunkedKey for ActorCommandKey {
	type ChunkKey = ActorCommandChunkKey;
	type Value = rivet_envoy_protocol::ActorCommandKeyData;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		ActorCommandChunkKey {
			namespace_id: self.namespace_id,
			envoy_key: self.envoy_key.clone(),
			actor_id: self.actor_id,
			generation: self.generation,
			index: self.index,
			chunk,
		}
	}

	fn combine(&self, chunks: Vec<Value>) -> Result<Self::Value> {
		rivet_envoy_protocol::versioned::ActorCommandKeyData::deserialize_with_embedded_version(
			&chunks
				.iter()
				.flat_map(|x| x.value().iter().copied())
				.collect::<Vec<_>>(),
		)
		.context("failed to combine `ActorCommandKey`")
	}

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
		Ok(
			rivet_envoy_protocol::versioned::ActorCommandKeyData::wrap_latest(value)
				.serialize_with_embedded_version(rivet_envoy_protocol::PROTOCOL_VERSION)?
				.chunks(universaldb::utils::CHUNK_SIZE)
				.map(|x| x.to_vec())
				.collect(),
		)
	}
}

/// Decodes a range read over the actor command subspace into commands in key order.
///
/// Commands are stored as chunks under `ActorCommandChunkKey`. Commands written before chunking hold
/// their entire value at the bare `ActorCommandKey`, so both layouts are accepted.
pub fn decode_actor_commands(
	tx: &universaldb::Transaction,
	entries: Vec<Value>,
) -> Result<Vec<(ActorCommandKey, rivet_envoy_protocol::ActorCommandKeyData)>> {
	let mut commands = Vec::new();
	let mut pending: Option<(ActorCommandKey, Vec<Value>)> = None;

	for entry in entries {
		match tx.unpack::<ActorCommandChunkKey>(entry.key()) {
			Ok(chunk_key) => {
				let key = chunk_key.into_command_key();
				match pending.as_mut() {
					Some((pending_key, chunks)) if pending_key.is_same_command(&key) => {
						chunks.push(entry);
					}
					_ => {
						flush_pending_command(&mut commands, pending.take())?;
						pending = Some((key, vec![entry]));
					}
				}
			}
			Err(_) => {
				flush_pending_command(&mut commands, pending.take())?;

				let key = tx.unpack::<ActorCommandKey>(entry.key())?;
				let command =
					rivet_envoy_protocol::versioned::ActorCommandKeyData::deserialize_with_embedded_version(
						entry.value(),
					)
					.context("failed to deserialize unchunked `ActorCommandKey`")?;
				commands.push((key, command));
			}
		}
	}

	flush_pending_command(&mut commands, pending)?;

	Ok(commands)
}

/// Bytes read per transaction before `read_actor_commands` ends the page at the next command.
pub const ACTOR_COMMAND_PAGE_BYTES: usize = util::size::mebibytes(1) as usize;
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

/// Reads every pending command for an envoy in key order across as many transactions as needed.
///
/// Each transaction ends at the next command once it has read `page_bytes` or run past
/// `EARLY_TXN_TIMEOUT`, so a large backlog cannot exceed the transaction time limit and a chunked
/// command is never split across pages.
pub async fn read_actor_commands(
	udb: &universaldb::Database,
	namespace_id: Id,
	envoy_key: &str,
	page_bytes: usize,
) -> Result<Vec<(ActorCommandKey, rivet_envoy_protocol::ActorCommandKeyData)>> {
	let subspace = crate::keys::subspace().subspace(&ActorCommandKey::subspace(
		namespace_id,
		envoy_key.to_string(),
	));
	let (mut cursor, range_end) = subspace.range();
	let mut commands = Vec::new();

	loop {
		let (page, next_cursor) = udb
			.txn("pegboard_envoy_read_actor_commands", |tx| {
				let cursor = &cursor;
				let range_end = &range_end;
				async move {
					let start = Instant::now();
					let tx = tx.with_subspace(crate::keys::subspace());
					let mut stream = tx.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(cursor.as_slice(), range_end.as_slice()).into()
						},
						Serializable,
					);
					let mut entries = Vec::new();
					let mut read_bytes = 0;

					let next_cursor = loop {
						let Some(entry) = stream.try_next().await? else {
							break None;
						};

						// Always take at least one full command so every page makes progress.
						if !entries.is_empty()
							&& (read_bytes >= page_bytes || start.elapsed() > EARLY_TXN_TIMEOUT)
							&& is_actor_command_start(&tx, entry.key())
						{
							break Some(entry.key().to_vec());
						}

						read_bytes += entry.key().len() + entry.value().len();
						entries.push(entry);
					};

					Ok((decode_actor_commands(&tx, entries)?, next_cursor))
				}
			})
			.custom_instrument(tracing::info_span!("read_actor_commands_tx"))
			.await?;

		commands.extend(page);

		match next_cursor {
			Some(next_cursor) => cursor = next_cursor,
			None => return Ok(commands),
		}
	}
}

/// Returns whether `key` starts a command, meaning it is the first chunk of a chunked value or an
/// unchunked value.
fn is_actor_command_start(tx: &universaldb::Transaction, key: &[u8]) -> bool {
	match tx.unpack::<ActorCommandChunkKey>(key) {
		Ok(chunk_key) => chunk_key.chunk == 0,
		Err(_) => true,
	}
}

fn flush_pending_command(
	commands: &mut Vec<(ActorCommandKey, rivet_envoy_protocol::ActorCommandKeyData)>,
	pending: Option<(ActorCommandKey, Vec<Value>)>,
) -> Result<()> {
	if let Some((key, chunks)) = pending {
		let command = key.combine(chunks)?;
		commands.push((key, command));
	}

	Ok(())
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

#[derive(Debug)]
pub struct ActorCommandChunkKey {
	namespace_id: Id,
	envoy_key: String,
	actor_id: Id,
	generation: u32,
	index: i64,
	chunk: usize,
}

impl ActorCommandChunkKey {
	fn into_command_key(self) -> ActorCommandKey {
		ActorCommandKey {
			namespace_id: self.namespace_id,
			envoy_key: self.envoy_key,
			actor_id: self.actor_id,
			generation: self.generation,
			index: self.index,
		}
	}
}

impl TuplePack for ActorCommandChunkKey {
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
			self.chunk,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorCommandChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, _, namespace_id, envoy_key, _, _, actor_id, generation, index, chunk)) =
			<(
				usize,
				usize,
				usize,
				Id,
				String,
				usize,
				usize,
				Id,
				u32,
				i64,
				usize,
			)>::unpack(input, tuple_depth)?;
		let v = ActorCommandChunkKey {
			namespace_id,
			envoy_key,
			actor_id,
			generation,
			index,
			chunk,
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
