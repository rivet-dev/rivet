//! Commands on the migration volume. Every write carries the writer's singleton tenure
//! (`coordinatorGeneration`) and commits only when no newer tenure wrote first, so a former holder
//! that keeps running after it lost the lease cannot change the state.
//!
//! The fence check and the writes are one command transaction on the volume's host: an attached
//! epoch alone could not stop a write that was already sent.

use std::ops::Bound;

use anyhow::{Context, Result};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::KvHandle;
use rivet_data::families::VersionFamily;
use rivet_data::generated::pegboard_v2_migrator_v2 as wire;
use rivet_keys::keys::{
	KeyAddress, SubspaceRange,
	codec::{AtVersion, FormalKey},
	tags::depot_command::{PEGBOARD_V2_MIGRATOR_STATE_LOAD, PEGBOARD_V2_MIGRATOR_STATE_WRITE},
};
use rivet_util::Id;

use pegboard::{keys::partition::V2MigratorVolume, migrate_from_v2::MigrateFromV2StateRead};

use crate::keys::kv::{
	V2MigratorFenceKey, V2MigratorInFlightKey, V2MigratorInFlightPrefix, V2MigratorPassKey,
};

/// Writes the pass entry and in-flight entries of one tenure. Repeating it writes the same values, so a
/// retry after a lost reply is harmless.
pub struct MigrateFromV2StateWrite;

impl Command for MigrateFromV2StateWrite {
	const TAG: CommandTag = CommandTag(PEGBOARD_V2_MIGRATOR_STATE_WRITE);

	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::MigrateFromV2StateWriteRequest;
	type Response = rivet_data::versioned::MigrateFromV2StateWriteResponse;
}

/// Claims the state for a new tenure, raising the fence, and returns every entry. Repeating it with
/// the same tenure returns the same entries.
pub struct MigrateFromV2StateLoad;

impl Command for MigrateFromV2StateLoad {
	const TAG: CommandTag = CommandTag(PEGBOARD_V2_MIGRATOR_STATE_LOAD);

	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::MigrateFromV2StateLoadRequest;
	type Response = rivet_data::versioned::MigrateFromV2StateLoadResponse;
}

/// Operator reads and the reads of namespaces whose backfill waits on the migrator are rare, so one
/// runs at a time.
const STATE_READ_CONCURRENCY: usize = 1;

pub fn register(registry: &mut depot_worker::command::CommandRegistry) -> Result<()> {
	registry.register::<MigrateFromV2StateWrite>(state_write)?;
	registry.register::<MigrateFromV2StateLoad>(state_load)?;
	registry.register_snapshot_read::<MigrateFromV2StateRead>(
		STATE_READ_CONCURRENCY,
		None,
		state_read,
	)?;
	Ok(())
}

/// The fence a write must meet: its tenure is at least the stored one.
async fn read_fence(txn: &mut depot_worker::command::Transaction) -> Result<u64> {
	let key = V2MigratorFenceKey::new();
	let Some(raw) = txn.get(&key.pack()).await? else {
		return Ok(0);
	};
	Ok(key.deserialize(raw.into())?.coordinator_generation)
}

async fn state_write(
	kv: KvHandle,
	request: wire::MigrateFromV2StateWriteRequest,
) -> Result<wire::MigrateFromV2StateWriteResponse> {
	V2MigratorVolume::from_volume_key(kv.volume_key())?;

	let version = kv.write_version(VersionFamily::PEGBOARD_V2_MIGRATOR)?;
	let mut txn = kv.begin().await?;
	let fence = read_fence(&mut txn).await?;
	if fence > request.coordinator_generation {
		return Ok(
			wire::MigrateFromV2StateWriteResponse::MigrateFromV2StateFenced(
				wire::MigrateFromV2StateFenced {
					coordinator_generation: fence,
				},
			),
		);
	}

	if let Some(pass) = request.pass {
		let key = V2MigratorPassKey::new();
		txn.put(&key.pack(), &key.serialize(AtVersion::new(pass, version))?)?;
	}
	for entry in request.puts {
		let v2_actor_id =
			Id::from_slice(&entry.candidate.v2_actor_id).context("in-flight actor id")?;
		let key = V2MigratorInFlightKey::new(v2_actor_id);
		// An entry placed in a volume group is inexpressible at version 1, and the write fails
		// while a storage worker that reads only version 1 is live. The migrator retries it after
		// a backoff; a namespace gains its group only once such workers are gone.
		txn.put(&key.pack(), &key.serialize(AtVersion::new(entry, version))?)?;
	}
	for v2_actor_id in request.deletes {
		let v2_actor_id = Id::from_slice(&v2_actor_id).context("deleted actor id")?;
		txn.delete(&V2MigratorInFlightKey::new(v2_actor_id).pack())?;
	}
	kv.commit(txn).await?;

	Ok(wire::MigrateFromV2StateWriteResponse::MigrateFromV2StateWritten)
}

async fn state_load(
	kv: KvHandle,
	request: wire::MigrateFromV2StateLoadRequest,
) -> Result<wire::MigrateFromV2StateLoadResponse> {
	V2MigratorVolume::from_volume_key(kv.volume_key())?;

	let mut txn = kv.begin().await?;
	let fence = read_fence(&mut txn).await?;
	if fence > request.coordinator_generation {
		return Ok(
			wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateFenced(
				wire::MigrateFromV2StateFenced {
					coordinator_generation: fence,
				},
			),
		);
	}

	let pass_key = V2MigratorPassKey::new();
	let pass = match txn.get(&pass_key.pack()).await? {
		Some(raw) => Some(pass_key.deserialize(raw.into())?),
		None => None,
	};

	// The in-flight entries are bounded by `max_in_flight`, so one read returns them all.
	let range = V2MigratorInFlightPrefix::new().range();
	let entries = txn
		.scan(
			(
				Bound::Included(range.start.as_slice()),
				Bound::Excluded(range.end.as_slice()),
			),
			None,
			false,
		)
		.await?;
	let in_flight = entries
		.into_iter()
		.map(|(raw_key, raw_value)| {
			let key = V2MigratorInFlightKey::unpack(&raw_key).context("invalid in-flight key")?;
			key.deserialize(raw_value.into())
		})
		.collect::<Result<Vec<_>>>()?;

	// Raise the fence so every older tenure's later writes are refused.
	if fence < request.coordinator_generation {
		let key = V2MigratorFenceKey::new();
		let version = kv.write_version(VersionFamily::PEGBOARD_V2_MIGRATOR)?;
		let fence = wire::MigrateFromV2Fence {
			coordinator_generation: request.coordinator_generation,
		};
		txn.put(&key.pack(), &key.serialize(AtVersion::new(fence, version))?)?;
		kv.commit(txn).await?;
	}

	Ok(
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(
			wire::MigrateFromV2StateLoaded { pass, in_flight },
		),
	)
}

async fn state_read(kv: KvHandle, _request: ()) -> Result<wire::MigrateFromV2StateLoadResponse> {
	V2MigratorVolume::from_volume_key(kv.volume_key())?;

	let mut snapshot = kv.snapshot().await?;
	let pass_key = V2MigratorPassKey::new();
	let pass = match snapshot.get(&pass_key.pack()).await? {
		Some(raw) => Some(pass_key.deserialize(raw.into())?),
		None => None,
	};
	let range = V2MigratorInFlightPrefix::new().range();
	let entries = snapshot
		.scan(
			(
				Bound::Included(range.start.as_slice()),
				Bound::Excluded(range.end.as_slice()),
			),
			None,
			false,
		)
		.await?;
	let in_flight = entries
		.into_iter()
		.map(|(raw_key, raw_value)| {
			let key = V2MigratorInFlightKey::unpack(&raw_key).context("invalid in-flight key")?;
			key.deserialize(raw_value.into())
		})
		.collect::<Result<Vec<_>>>()?;

	Ok(
		wire::MigrateFromV2StateLoadResponse::MigrateFromV2StateLoaded(
			wire::MigrateFromV2StateLoaded { pass, in_flight },
		),
	)
}
