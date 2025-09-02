use anyhow::*;
use epoxy_protocol::protocol::{ReplicaId, SlotId};
use udb_util::prelude::*;
use versioned_data_util::OwnedVersionedData as _;

#[derive(Debug)]
pub struct InstanceNumberKey;

impl FormalKey for InstanceNumberKey {
	type Value = u64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for InstanceNumberKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (INSTANCE_NUMBER,);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct LogEntryKey {
	pub instance_replica_id: ReplicaId,
	pub instance_slot_id: SlotId,
}

impl LogEntryKey {
	pub fn new(instance_replica_id: ReplicaId, instance_slot_id: SlotId) -> Self {
		Self {
			instance_replica_id,
			instance_slot_id,
		}
	}
}

impl FormalKey for LogEntryKey {
	type Value = epoxy_protocol::protocol::LogEntry;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		epoxy_protocol::versioned::LogEntry::deserialize_with_embedded_version(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		epoxy_protocol::versioned::LogEntry::latest(value)
			.serialize_with_embedded_version(epoxy_protocol::PROTOCOL_VERSION)
	}
}

impl TuplePack for LogEntryKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (LOG, self.instance_replica_id, self.instance_slot_id, ENTRY);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for LogEntryKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, instance_replica_id, instance_slot_id, _)) =
			<(usize, ReplicaId, SlotId, usize)>::unpack(input, tuple_depth)?;

		let v = LogEntryKey {
			instance_replica_id,
			instance_slot_id,
		};

		Result::Ok((input, v))
	}
}

#[derive(Debug)]
pub struct KeyInstanceKey {
	pub key: Vec<u8>,
	pub instance_replica_id: ReplicaId,
	pub instance_slot_id: SlotId,
}

impl KeyInstanceKey {
	pub fn new(key: Vec<u8>, instance_replica_id: ReplicaId, instance_slot_id: SlotId) -> Self {
		Self {
			key,
			instance_replica_id,
			instance_slot_id,
		}
	}

	pub fn subspace(key: Vec<u8>) -> KeyInstanceSubspaceKey {
		KeyInstanceSubspaceKey::new(key)
	}

	pub fn subspace_for_all_keys() -> KeyInstanceSubspaceForAllKeysKey {
		KeyInstanceSubspaceForAllKeysKey
	}
}

impl FormalKey for KeyInstanceKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for KeyInstanceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			KEY_INSTANCE,
			&self.key,
			INSTANCE,
			self.instance_replica_id,
			self.instance_slot_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for KeyInstanceKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, key, _, instance_replica_id, instance_slot_id)) =
			<(usize, Vec<u8>, usize, ReplicaId, SlotId)>::unpack(input, tuple_depth)?;

		let v = KeyInstanceKey {
			key,
			instance_replica_id,
			instance_slot_id,
		};

		Result::Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ConfigKey;

impl FormalKey for ConfigKey {
	type Value = epoxy_protocol::protocol::ClusterConfig;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		epoxy_protocol::versioned::ClusterConfig::deserialize_with_embedded_version(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		epoxy_protocol::versioned::ClusterConfig::latest(value)
			.serialize_with_embedded_version(epoxy_protocol::PROTOCOL_VERSION)
	}
}

impl TuplePack for ConfigKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CONFIG,);
		t.pack(w, tuple_depth)
	}
}

pub struct KeyInstanceSubspaceKey {
	key: Vec<u8>,
}

impl KeyInstanceSubspaceKey {
	pub fn new(key: Vec<u8>) -> Self {
		KeyInstanceSubspaceKey { key }
	}
}

impl TuplePack for KeyInstanceSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (KEY_INSTANCE, &self.key, INSTANCE);
		t.pack(w, tuple_depth)
	}
}

pub struct KeyInstanceSubspaceForAllKeysKey;

impl TuplePack for KeyInstanceSubspaceForAllKeysKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (KEY_INSTANCE,);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct CurrentBallotKey;

impl FormalKey for CurrentBallotKey {
	type Value = epoxy_protocol::protocol::Ballot;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		epoxy_protocol::versioned::Ballot::deserialize_with_embedded_version(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		epoxy_protocol::versioned::Ballot::latest(value)
			.serialize_with_embedded_version(epoxy_protocol::PROTOCOL_VERSION)
	}
}

impl TuplePack for CurrentBallotKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CURRENT_BALLOT,);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct InstanceBallotKey {
	instance_replica_id: ReplicaId,
	instance_slot_id: SlotId,
}

impl InstanceBallotKey {
	pub fn new(instance_replica_id: ReplicaId, instance_slot_id: SlotId) -> Self {
		Self {
			instance_replica_id,
			instance_slot_id,
		}
	}
}

impl FormalKey for InstanceBallotKey {
	type Value = epoxy_protocol::protocol::Ballot;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		epoxy_protocol::versioned::Ballot::deserialize_with_embedded_version(raw)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		epoxy_protocol::versioned::Ballot::latest(value)
			.serialize_with_embedded_version(epoxy_protocol::PROTOCOL_VERSION)
	}
}

impl TuplePack for InstanceBallotKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			INSTANCE_BALLOT,
			self.instance_replica_id,
			self.instance_slot_id,
		);
		t.pack(w, tuple_depth)
	}
}
