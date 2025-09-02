use std::result::Result::Ok;

use anyhow::*;
use gas::prelude::*;
use udb_util::prelude::*;
use versioned_data_util::OwnedVersionedData;

#[derive(Debug)]
pub struct RunnerAllocIdxKey {
	pub namespace_id: Id,
	pub name: String,
	pub version: u32,
	pub remaining_millislots: u32,
	pub last_ping_ts: i64,
	pub runner_id: Id,
}

impl RunnerAllocIdxKey {
	pub fn new(
		namespace_id: Id,
		name: String,
		version: u32,
		remaining_millislots: u32,
		last_ping_ts: i64,
		runner_id: Id,
	) -> Self {
		RunnerAllocIdxKey {
			namespace_id,
			name,
			version,
			remaining_millislots,
			last_ping_ts,
			runner_id,
		}
	}

	pub fn subspace(namespace_id: Id, name: String) -> RunnerAllocIdxSubspaceKey {
		RunnerAllocIdxSubspaceKey::new(namespace_id, name)
	}

	pub fn entire_subspace() -> RunnerAllocIdxSubspaceKey {
		RunnerAllocIdxSubspaceKey::entire()
	}
}

impl FormalKey for RunnerAllocIdxKey {
	type Value = rivet_key_data::converted::RunnerAllocIdxKeyData;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		rivet_key_data::versioned::RunnerAllocIdxKeyData::deserialize_with_embedded_version(raw)?
			.try_into()
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		rivet_key_data::versioned::RunnerAllocIdxKeyData::latest(value.try_into()?)
			.serialize_with_embedded_version(
				rivet_key_data::PEGBOARD_DATACENTER_RUNNER_ALLOC_IDX_VERSION,
			)
	}
}

impl TuplePack for RunnerAllocIdxKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			DATACENTER,
			RUNNER_ALLOC_IDX,
			self.namespace_id,
			&self.name,
			// Stored in reverse order (higher versions are first)
			-(self.version as i32),
			// Stored in reverse order (higher remaining slots are first)
			-(self.remaining_millislots as i32),
			self.last_ping_ts,
			self.runner_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerAllocIdxKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (
			input,
			(_, _, namespace_id, name, version, remaining_millislots, last_ping_ts, runner_id),
		) = <(usize, usize, Id, String, i32, i32, i64, Id)>::unpack(input, tuple_depth)?;

		let v = RunnerAllocIdxKey {
			namespace_id,
			name,
			version: -version as u32,
			remaining_millislots: -remaining_millislots as u32,
			last_ping_ts,
			runner_id,
		};

		Ok((input, v))
	}
}

pub struct RunnerAllocIdxSubspaceKey {
	pub namespace_id: Option<Id>,
	pub name: Option<String>,
}

impl RunnerAllocIdxSubspaceKey {
	pub fn new(namespace_id: Id, name: String) -> Self {
		RunnerAllocIdxSubspaceKey {
			namespace_id: Some(namespace_id),
			name: Some(name),
		}
	}

	pub fn entire() -> Self {
		RunnerAllocIdxSubspaceKey {
			namespace_id: None,
			name: None,
		}
	}
}

impl TuplePack for RunnerAllocIdxSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (DATACENTER, RUNNER_ALLOC_IDX);
		offset += t.pack(w, tuple_depth)?;

		if let Some(namespace_id) = &self.namespace_id {
			offset += namespace_id.pack(w, tuple_depth)?;

			if let Some(name) = &self.name {
				offset += name.pack(w, tuple_depth)?;
			}
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct PendingActorByRunnerNameSelectorKey {
	pub namespace_id: Id,
	pub runner_name_selector: String,
	pub ts: i64,
	pub actor_id: Id,
}

impl PendingActorByRunnerNameSelectorKey {
	pub fn new(namespace_id: Id, runner_name_selector: String, ts: i64, actor_id: Id) -> Self {
		PendingActorByRunnerNameSelectorKey {
			namespace_id,
			runner_name_selector,
			ts,
			actor_id,
		}
	}

	pub fn subspace(
		namespace_id: Id,
		runner_name_selector: String,
	) -> PendingActorByRunnerNameSelectorSubspaceKey {
		PendingActorByRunnerNameSelectorSubspaceKey::new(namespace_id, runner_name_selector)
	}
}

impl FormalKey for PendingActorByRunnerNameSelectorKey {
	/// Generation.
	type Value = u32;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(u32::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for PendingActorByRunnerNameSelectorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			DATACENTER,
			PENDING_ACTOR_BY_RUNNER_NAME_SELECTOR,
			self.namespace_id,
			&self.runner_name_selector,
			self.ts,
			self.actor_id,
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for PendingActorByRunnerNameSelectorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, namespace_id, runner_name_selector, ts, actor_id)) =
			<(usize, usize, Id, String, i64, Id)>::unpack(input, tuple_depth)?;

		let v = PendingActorByRunnerNameSelectorKey {
			namespace_id,
			runner_name_selector,
			ts,
			actor_id,
		};

		Ok((input, v))
	}
}

pub struct PendingActorByRunnerNameSelectorSubspaceKey {
	pub namespace_id: Id,
	pub runner_name_selector: String,
}

impl PendingActorByRunnerNameSelectorSubspaceKey {
	pub fn new(namespace_id: Id, runner_name_selector: String) -> Self {
		PendingActorByRunnerNameSelectorSubspaceKey {
			namespace_id,
			runner_name_selector,
		}
	}
}

impl TuplePack for PendingActorByRunnerNameSelectorSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			DATACENTER,
			PENDING_ACTOR_BY_RUNNER_NAME_SELECTOR,
			self.namespace_id,
			&self.runner_name_selector,
		);
		t.pack(w, tuple_depth)
	}
}
