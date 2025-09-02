use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;
use versioned_data_util::OwnedVersionedData;

#[derive(Debug)]
pub struct ActiveActorKey {
	namespace_id: Id,
	pub name: String,
	pub create_ts: i64,
	pub actor_id: Id,
}

impl ActiveActorKey {
	pub fn new(namespace_id: Id, name: String, create_ts: i64, actor_id: Id) -> Self {
		ActiveActorKey {
			namespace_id,
			name,
			create_ts,
			actor_id,
		}
	}

	pub fn subspace(namespace_id: Id, name: String) -> ActiveActorSubspaceKey {
		ActiveActorSubspaceKey::new(namespace_id, name)
	}

	pub fn subspace_with_create_ts(
		namespace_id: Id,
		name: String,
		create_ts: i64,
	) -> ActiveActorSubspaceKey {
		ActiveActorSubspaceKey::new_with_create_ts(namespace_id, name, create_ts)
	}
}

impl FormalKey for ActiveActorKey {
	/// Workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for ActiveActorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			ACTOR,
			BY_NAME,
			ACTIVE,
			&self.name,
			self.create_ts,
			self.actor_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActiveActorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, _, name, create_ts, actor_id)) =
			<(usize, Id, usize, usize, usize, String, i64, Id)>::unpack(input, tuple_depth)?;
		let v = ActiveActorKey {
			namespace_id,
			name,
			create_ts,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct ActiveActorSubspaceKey {
	namespace_id: Id,
	name: String,
	create_ts: Option<i64>,
}

impl ActiveActorSubspaceKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		ActiveActorSubspaceKey {
			namespace_id,
			name,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> Self {
		ActiveActorSubspaceKey {
			namespace_id,
			name,
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for ActiveActorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			NAMESPACE,
			self.namespace_id,
			ACTOR,
			BY_NAME,
			ACTIVE,
			&self.name,
		);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct AllActorKey {
	namespace_id: Id,
	pub name: String,
	pub create_ts: i64,
	pub actor_id: Id,
}

impl AllActorKey {
	pub fn new(namespace_id: Id, name: String, create_ts: i64, actor_id: Id) -> Self {
		AllActorKey {
			namespace_id,
			name,
			create_ts,
			actor_id,
		}
	}

	pub fn subspace(namespace_id: Id, name: String) -> AllActorSubspaceKey {
		AllActorSubspaceKey::new(namespace_id, name)
	}

	pub fn subspace_with_create_ts(
		namespace_id: Id,
		name: String,
		create_ts: i64,
	) -> AllActorSubspaceKey {
		AllActorSubspaceKey::new_with_create_ts(namespace_id, name, create_ts)
	}
}

impl FormalKey for AllActorKey {
	/// Workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for AllActorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			ACTOR,
			BY_NAME,
			ALL,
			&self.name,
			self.create_ts,
			self.actor_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AllActorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, _, name, create_ts, actor_id)) =
			<(usize, Id, usize, usize, usize, String, i64, Id)>::unpack(input, tuple_depth)?;
		let v = AllActorKey {
			namespace_id,
			name,
			create_ts,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct AllActorSubspaceKey {
	namespace_id: Id,
	name: String,
	create_ts: Option<i64>,
}

impl AllActorSubspaceKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		AllActorSubspaceKey {
			namespace_id,
			name,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> Self {
		AllActorSubspaceKey {
			namespace_id,
			name,
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for AllActorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			NAMESPACE,
			self.namespace_id,
			ACTOR,
			BY_NAME,
			ALL,
			&self.name,
		);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct ActorByKeyKey {
	namespace_id: Id,
	pub name: String,
	pub k: String,
	pub create_ts: i64,
	pub actor_id: Id,
}

impl ActorByKeyKey {
	pub fn new(namespace_id: Id, name: String, k: String, create_ts: i64, actor_id: Id) -> Self {
		ActorByKeyKey {
			namespace_id,
			name,
			k,
			create_ts,
			actor_id,
		}
	}

	pub fn subspace(namespace_id: Id, name: String, k: String) -> ActorByKeySubspaceKey {
		ActorByKeySubspaceKey::new(namespace_id, name, k)
	}

	pub fn subspace_with_create_ts(
		namespace_id: Id,
		name: String,
		k: String,
		create_ts: i64,
	) -> ActorByKeySubspaceKey {
		ActorByKeySubspaceKey::new_with_create_ts(namespace_id, name, k, create_ts)
	}

	pub fn null(namespace_id: Id, name: String, create_ts: i64, actor_id: Id) -> Self {
		ActorByKeyKey {
			namespace_id,
			name,
			k: String::new(),
			create_ts,
			actor_id,
		}
	}

	pub fn null_subspace(namespace_id: Id, name: String) -> ActorByKeySubspaceKey {
		ActorByKeySubspaceKey::null(namespace_id, name)
	}

	pub fn null_subspace_with_create_ts(
		namespace_id: Id,
		name: String,
		create_ts: i64,
	) -> ActorByKeySubspaceKey {
		ActorByKeySubspaceKey::null_with_create_ts(namespace_id, name, create_ts)
	}
}

impl FormalKey for ActorByKeyKey {
	type Value = rivet_key_data::converted::ActorByKeyKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		rivet_key_data::versioned::ActorByKeyKeyData::deserialize_with_embedded_version(raw)?
			.try_into()
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_key_data::versioned::ActorByKeyKeyData::latest(value.try_into()?)
			.serialize_with_embedded_version(
				rivet_key_data::PEGBOARD_NAMESPACE_ACTOR_BY_KEY_VERSION,
			)
	}
}

impl TuplePack for ActorByKeyKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			ACTOR,
			BY_NAME_AND_KEY,
			&self.name,
			&self.k,
			self.create_ts,
			self.actor_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorByKeyKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, name, k, create_ts, actor_id)) =
			<(usize, Id, usize, usize, String, String, i64, Id)>::unpack(input, tuple_depth)?;
		let v = ActorByKeyKey {
			namespace_id,
			name,
			k,
			create_ts,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct ActorByKeySubspaceKey {
	namespace_id: Id,
	name: String,
	k: String,
	create_ts: Option<i64>,
}

impl ActorByKeySubspaceKey {
	pub fn new(namespace_id: Id, name: String, k: String) -> Self {
		ActorByKeySubspaceKey {
			namespace_id,
			name,
			k,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, name: String, k: String, create_ts: i64) -> Self {
		ActorByKeySubspaceKey {
			namespace_id,
			name,
			k,
			create_ts: Some(create_ts),
		}
	}

	pub fn null(namespace_id: Id, name: String) -> Self {
		ActorByKeySubspaceKey {
			namespace_id,
			name,
			k: String::new(),
			create_ts: None,
		}
	}

	pub fn null_with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> Self {
		ActorByKeySubspaceKey {
			namespace_id,
			name,
			k: String::new(),
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for ActorByKeySubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			NAMESPACE,
			self.namespace_id,
			ACTOR,
			BY_NAME_AND_KEY,
			&self.name,
			&self.k,
		);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct ActiveRunnerKey {
	namespace_id: Id,
	pub create_ts: i64,
	pub runner_id: Id,
}

impl ActiveRunnerKey {
	pub fn new(namespace_id: Id, create_ts: i64, runner_id: Id) -> Self {
		ActiveRunnerKey {
			namespace_id,
			create_ts,
			runner_id,
		}
	}

	pub fn subspace(namespace_id: Id) -> ActiveRunnerSubspaceKey {
		ActiveRunnerSubspaceKey::new(namespace_id)
	}

	pub fn subspace_with_create_ts(namespace_id: Id, create_ts: i64) -> ActiveRunnerSubspaceKey {
		ActiveRunnerSubspaceKey::new_with_create_ts(namespace_id, create_ts)
	}
}

impl FormalKey for ActiveRunnerKey {
	/// Workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for ActiveRunnerKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			ACTIVE,
			self.create_ts,
			self.runner_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActiveRunnerKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, create_ts, runner_id)) =
			<(usize, Id, usize, usize, i64, Id)>::unpack(input, tuple_depth)?;
		let v = ActiveRunnerKey {
			namespace_id,
			create_ts,
			runner_id,
		};

		Ok((input, v))
	}
}

pub struct ActiveRunnerSubspaceKey {
	namespace_id: Id,
	create_ts: Option<i64>,
}

impl ActiveRunnerSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		ActiveRunnerSubspaceKey {
			namespace_id,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, create_ts: i64) -> Self {
		ActiveRunnerSubspaceKey {
			namespace_id,
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for ActiveRunnerSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (NAMESPACE, self.namespace_id, RUNNER, ACTIVE);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct AllRunnerKey {
	namespace_id: Id,
	pub create_ts: i64,
	pub runner_id: Id,
}

impl AllRunnerKey {
	pub fn new(namespace_id: Id, create_ts: i64, runner_id: Id) -> Self {
		AllRunnerKey {
			namespace_id,
			create_ts,
			runner_id,
		}
	}

	pub fn subspace(namespace_id: Id) -> AllRunnerSubspaceKey {
		AllRunnerSubspaceKey::new(namespace_id)
	}

	pub fn subspace_with_create_ts(namespace_id: Id, create_ts: i64) -> AllRunnerSubspaceKey {
		AllRunnerSubspaceKey::new_with_create_ts(namespace_id, create_ts)
	}
}

impl FormalKey for AllRunnerKey {
	/// Workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for AllRunnerKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			ALL,
			self.create_ts,
			self.runner_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AllRunnerKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, create_ts, runner_id)) =
			<(usize, Id, usize, usize, i64, Id)>::unpack(input, tuple_depth)?;
		let v = AllRunnerKey {
			namespace_id,
			create_ts,
			runner_id,
		};

		Ok((input, v))
	}
}

pub struct AllRunnerSubspaceKey {
	namespace_id: Id,
	create_ts: Option<i64>,
}

impl AllRunnerSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		AllRunnerSubspaceKey {
			namespace_id,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, create_ts: i64) -> Self {
		AllRunnerSubspaceKey {
			namespace_id,
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for AllRunnerSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (NAMESPACE, self.namespace_id, RUNNER, ALL);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct ActiveRunnerByNameKey {
	namespace_id: Id,
	pub name: String,
	pub create_ts: i64,
	pub runner_id: Id,
}

impl ActiveRunnerByNameKey {
	pub fn new(namespace_id: Id, name: String, create_ts: i64, runner_id: Id) -> Self {
		ActiveRunnerByNameKey {
			namespace_id,
			name,
			create_ts,
			runner_id,
		}
	}

	pub fn subspace(namespace_id: Id, name: String) -> ActiveRunnerByNameSubspaceKey {
		ActiveRunnerByNameSubspaceKey::new(namespace_id, name)
	}

	pub fn subspace_with_create_ts(
		namespace_id: Id,
		name: String,
		create_ts: i64,
	) -> ActiveRunnerByNameSubspaceKey {
		ActiveRunnerByNameSubspaceKey::new_with_create_ts(namespace_id, name, create_ts)
	}
}

impl FormalKey for ActiveRunnerByNameKey {
	/// Workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for ActiveRunnerByNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			BY_NAME,
			ACTIVE,
			&self.name,
			self.create_ts,
			self.runner_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActiveRunnerByNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, _, name, create_ts, runner_id)) =
			<(usize, Id, usize, usize, usize, String, i64, Id)>::unpack(input, tuple_depth)?;
		let v = ActiveRunnerByNameKey {
			namespace_id,
			name,
			create_ts,
			runner_id,
		};

		Ok((input, v))
	}
}

pub struct ActiveRunnerByNameSubspaceKey {
	namespace_id: Id,
	name: String,
	create_ts: Option<i64>,
}

impl ActiveRunnerByNameSubspaceKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		ActiveRunnerByNameSubspaceKey {
			namespace_id,
			name,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> Self {
		ActiveRunnerByNameSubspaceKey {
			namespace_id,
			name,
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for ActiveRunnerByNameSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			BY_NAME,
			ACTIVE,
			&self.name,
		);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct AllRunnerByNameKey {
	namespace_id: Id,
	pub name: String,
	pub create_ts: i64,
	pub runner_id: Id,
}

impl AllRunnerByNameKey {
	pub fn new(namespace_id: Id, name: String, create_ts: i64, runner_id: Id) -> Self {
		AllRunnerByNameKey {
			namespace_id,
			name,
			create_ts,
			runner_id,
		}
	}

	pub fn subspace(namespace_id: Id, name: String) -> AllRunnerByNameSubspaceKey {
		AllRunnerByNameSubspaceKey::new(namespace_id, name)
	}

	pub fn subspace_with_create_ts(
		namespace_id: Id,
		name: String,
		create_ts: i64,
	) -> AllRunnerByNameSubspaceKey {
		AllRunnerByNameSubspaceKey::new_with_create_ts(namespace_id, name, create_ts)
	}
}

impl FormalKey for AllRunnerByNameKey {
	/// Workflow id.
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for AllRunnerByNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			BY_NAME,
			ALL,
			&self.name,
			self.create_ts,
			self.runner_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for AllRunnerByNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, _, name, create_ts, runner_id)) =
			<(usize, Id, usize, usize, usize, String, i64, Id)>::unpack(input, tuple_depth)?;
		let v = AllRunnerByNameKey {
			namespace_id,
			name,
			create_ts,
			runner_id,
		};

		Ok((input, v))
	}
}

pub struct AllRunnerByNameSubspaceKey {
	namespace_id: Id,
	name: String,
	create_ts: Option<i64>,
}

impl AllRunnerByNameSubspaceKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		AllRunnerByNameSubspaceKey {
			namespace_id,
			name,
			create_ts: None,
		}
	}

	pub fn new_with_create_ts(namespace_id: Id, name: String, create_ts: i64) -> Self {
		AllRunnerByNameSubspaceKey {
			namespace_id,
			name,
			create_ts: Some(create_ts),
		}
	}
}

impl TuplePack for AllRunnerByNameSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			BY_NAME,
			ALL,
			&self.name,
		);
		offset += t.pack(w, tuple_depth)?;

		if let Some(create_ts) = &self.create_ts {
			offset += create_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct RunnerByKeyKey {
	namespace_id: Id,
	pub name: String,
	pub k: String,
}

impl RunnerByKeyKey {
	pub fn new(namespace_id: Id, name: String, k: String) -> Self {
		RunnerByKeyKey {
			namespace_id,
			name,
			k,
		}
	}
}

impl FormalKey for RunnerByKeyKey {
	type Value = rivet_key_data::converted::RunnerByKeyKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		rivet_key_data::versioned::RunnerByKeyKeyData::deserialize_with_embedded_version(raw)?
			.try_into()
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_key_data::versioned::RunnerByKeyKeyData::latest(value.try_into()?)
			.serialize_with_embedded_version(
				rivet_key_data::PEGBOARD_NAMESPACE_RUNNER_BY_KEY_VERSION,
			)
	}
}

impl TuplePack for RunnerByKeyKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			NAMESPACE,
			self.namespace_id,
			RUNNER,
			BY_NAME_AND_KEY,
			&self.name,
			&self.k,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerByKeyKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, name, k)) =
			<(usize, Id, usize, usize, String, String)>::unpack(input, tuple_depth)?;
		let v = RunnerByKeyKey {
			namespace_id,
			name,
			k,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ActorNameKey {
	namespace_id: Id,
	pub name: String,
}

impl ActorNameKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		ActorNameKey { namespace_id, name }
	}

	pub fn subspace(namespace_id: Id) -> ActorNameSubspaceKey {
		ActorNameSubspaceKey::new(namespace_id)
	}
}

impl FormalKey for ActorNameKey {
	type Value = rivet_key_data::converted::ActorNameKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		rivet_key_data::versioned::ActorNameKeyData::deserialize_with_embedded_version(raw)?
			.try_into()
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_key_data::versioned::ActorNameKeyData::latest(value.try_into()?)
			.serialize_with_embedded_version(rivet_key_data::PEGBOARD_NAMESPACE_ACTOR_NAME_VERSION)
	}
}

impl TuplePack for ActorNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (NAMESPACE, self.namespace_id, ACTOR, NAME, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ActorNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, name)) =
			<(usize, Id, usize, usize, String)>::unpack(input, tuple_depth)?;

		let v = ActorNameKey { namespace_id, name };

		Ok((input, v))
	}
}

pub struct ActorNameSubspaceKey {
	namespace_id: Id,
}

impl ActorNameSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		ActorNameSubspaceKey { namespace_id }
	}
}

impl TuplePack for ActorNameSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (NAMESPACE, self.namespace_id, ACTOR, NAME);
		t.pack(w, tuple_depth)
	}
}

#[derive(Debug)]
pub struct RunnerNameKey {
	namespace_id: Id,
	pub name: String,
}

impl RunnerNameKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		RunnerNameKey { namespace_id, name }
	}

	pub fn subspace(namespace_id: Id) -> RunnerNameSubspaceKey {
		RunnerNameSubspaceKey::new(namespace_id)
	}
}

impl FormalKey for RunnerNameKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(vec![])
	}
}

impl TuplePack for RunnerNameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (NAMESPACE, self.namespace_id, RUNNER, NAME, &self.name);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerNameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, namespace_id, _, _, name)) =
			<(usize, Id, usize, usize, String)>::unpack(input, tuple_depth)?;

		let v = RunnerNameKey { namespace_id, name };

		Ok((input, v))
	}
}

pub struct RunnerNameSubspaceKey {
	namespace_id: Id,
}

impl RunnerNameSubspaceKey {
	pub fn new(namespace_id: Id) -> Self {
		RunnerNameSubspaceKey { namespace_id }
	}
}

impl TuplePack for RunnerNameSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (NAMESPACE, self.namespace_id, RUNNER, NAME);
		t.pack(w, tuple_depth)
	}
}
