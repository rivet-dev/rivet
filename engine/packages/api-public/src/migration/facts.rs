//! Reads the per-actor facts every migration route decides on.

use anyhow::{Context, Result};
use depot::{conveyer::branch as depot_branch, keys as depot_keys, types::BucketId};
use futures_util::TryStreamExt;
use pegboard::keys;
use rivet_util::Id;
use universaldb::{options::StreamingMode, utils::IsolationLevel};
use vbare::OwnedVersionedData;

use super::keys as mkeys;

/// The SQLite v1 (KV-backed) format stores its files under this KV key prefix.
const SQLITE_V1_PREFIX: u8 = 0x08;

/// Pegboard and gasoline state of one actor, as of one transaction's read version.
#[derive(Debug, Clone)]
pub struct ActorFacts {
	pub actor_id: Id,
	pub namespace_id: Id,
	pub workflow_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub pool_name: String,
	pub create_ts: i64,
	pub version: u32,
	pub generation: u32,
	pub sleep_ts: Option<i64>,
	/// The actor is connectable, or an envoy still holds it while it stops.
	pub allocated: bool,
	pub destroy_ts: Option<i64>,
	pub hibernating: bool,
	pub wake_deadline_ts: Option<i64>,
}

/// The oldest last ping a hibernating request may have and still hold its actor: V2's own
/// reads ignore requests the gateway stopped pinging for longer than the eligible threshold,
/// such as the ones a killed gateway leaves behind.
pub fn hibernating_since_ts(eligible_threshold_ms: i64) -> i64 {
	rivet_util::timestamp::now() - eligible_threshold_ms
}

/// Reads an actor's facts. Returns `None` if the actor does not exist. A hibernating request
/// last pinged before `hibernating_since_ts` does not count.
pub async fn read(
	tx: &universaldb::Transaction,
	actor_id: Id,
	hibernating_since_ts: i64,
	isolation: IsolationLevel,
) -> Result<Option<ActorFacts>> {
	let pb = tx.with_subspace(keys::subspace());

	let namespace_id_key = keys::actor::NamespaceIdKey::new(actor_id);
	let workflow_id_key = keys::actor::WorkflowIdKey::new(actor_id);
	let name_key = keys::actor::NameKey::new(actor_id);
	let key_key = keys::actor::KeyKey::new(actor_id);
	let pool_name_key = keys::actor::PoolNameKey::new(actor_id);
	let runner_name_selector_key = keys::actor::RunnerNameSelectorKey::new(actor_id);
	let create_ts_key = keys::actor::CreateTsKey::new(actor_id);
	let version_key = keys::actor::VersionKey::new(actor_id);
	let generation_key = keys::actor::GenerationKey::new(actor_id);
	let sleep_ts_key = keys::actor::SleepTsKey::new(actor_id);
	let envoy_key_key = keys::actor::EnvoyKeyKey::new(actor_id);
	let connectable_key = keys::actor::ConnectableKey::new(actor_id);
	let destroy_ts_key = keys::actor::DestroyTsKey::new(actor_id);

	let (
		namespace_id,
		workflow_id,
		name,
		key,
		pool_name,
		runner_name_selector,
		create_ts,
		version,
		generation,
		sleep_ts,
		envoy_key,
		connectable,
		destroy_ts,
		hibernating,
	) = tokio::try_join!(
		pb.read_opt(&namespace_id_key, isolation),
		pb.read_opt(&workflow_id_key, isolation),
		pb.read_opt(&name_key, isolation),
		pb.read_opt(&key_key, isolation),
		pb.read_opt(&pool_name_key, isolation),
		pb.read_opt(&runner_name_selector_key, isolation),
		pb.read_opt(&create_ts_key, isolation),
		pb.read_opt(&version_key, isolation),
		pb.read_opt(&generation_key, isolation),
		pb.read_opt(&sleep_ts_key, isolation),
		pb.exists(&envoy_key_key, isolation),
		pb.exists(&connectable_key, isolation),
		pb.read_opt(&destroy_ts_key, isolation),
		has_hibernating_request(&pb, actor_id, hibernating_since_ts, isolation),
	)?;

	let (Some(namespace_id), Some(workflow_id)) = (namespace_id, workflow_id) else {
		return Ok(None);
	};

	let wake_deadline_ts = read_wake_deadline(tx, workflow_id, isolation).await?;

	// A stopping actor keeps its envoy key until the envoy reports it stopped, so the key alone
	// marks an actor that may still be running. Gasoline commits an activity's writes apart from
	// its state change, so a lost state change can leave the key behind on a sleeping actor that
	// no envoy holds. The workflow's own state decides then, as it does for V2's lifecycle.
	let allocated =
		connectable || (envoy_key && workflow_allocated(tx, workflow_id, isolation).await?);

	Ok(Some(ActorFacts {
		actor_id,
		namespace_id,
		workflow_id,
		name: name.unwrap_or_default(),
		key,
		pool_name: pool_name.or(runner_name_selector).unwrap_or_default(),
		create_ts: create_ts.unwrap_or_default(),
		version: version.unwrap_or(1),
		generation: generation.unwrap_or(0),
		sleep_ts,
		allocated,
		destroy_ts,
		hibernating,
		wake_deadline_ts,
	}))
}

/// The allocation fields of the `pegboard_actor2` workflow state.
#[derive(serde::Deserialize)]
struct WorkflowAllocation {
	allocate_ts: Option<i64>,
	envoy_key: Option<String>,
}

/// Whether the actor workflow's state holds an allocation. A workflow with no state reads as
/// allocated, so the migration leaves an actor it cannot judge alone.
async fn workflow_allocated(
	tx: &universaldb::Transaction,
	workflow_id: Id,
	isolation: IsolationLevel,
) -> Result<bool> {
	let subspace = mkeys::workflow_state_subspace(workflow_id);
	let chunks = tx
		.get_ranges_keyvalues(
			universaldb::RangeOption {
				mode: StreamingMode::WantAll,
				..(&subspace).into()
			},
			isolation,
		)
		.try_collect::<Vec<_>>()
		.await?;
	if chunks.is_empty() {
		return Ok(true);
	}

	let state = chunks
		.iter()
		.flat_map(|chunk| chunk.value().iter().copied())
		.collect::<Vec<u8>>();
	let state = serde_json::from_slice::<WorkflowAllocation>(&state)
		.context("actor workflow state has no allocation fields")?;
	Ok(state.allocate_ts.is_some() || state.envoy_key.is_some())
}

async fn has_hibernating_request(
	pb: &universaldb::Transaction,
	actor_id: Id,
	since_ts: i64,
	isolation: IsolationLevel,
) -> Result<bool> {
	// Rows are ordered by last ping, so the range starts at the oldest live one, as V2's own
	// hibernating request list does.
	let start = pb.pack(&keys::actor::HibernatingRequestKey::subspace_with_ts(
		actor_id, since_ts,
	));
	let end = keys::subspace()
		.subspace(&keys::actor::HibernatingRequestKey::subspace(actor_id))
		.range()
		.1;
	let mut stream = pb.get_ranges_keyvalues(
		universaldb::RangeOption {
			limit: Some(1),
			mode: StreamingMode::Small,
			..(start, end).into()
		},
		isolation,
	);

	Ok(stream.try_next().await?.is_some())
}

/// The workflow's pending deadline, if it has one.
///
/// Gasoline keeps a workflow's last deadline in its wake-deadline key after the deadline is
/// consumed: a commit without a new deadline deletes the deadline's wake row but not the key. So
/// the key is a pending deadline only while its wake row exists; otherwise a fired alarm would
/// read as one that is about to fire on every pass.
pub async fn read_wake_deadline(
	tx: &universaldb::Transaction,
	workflow_id: Id,
	isolation: IsolationLevel,
) -> Result<Option<i64>> {
	let Some(raw) = tx
		.get(&mkeys::wake_deadline(workflow_id), isolation)
		.await?
	else {
		return Ok(None);
	};

	let bytes: [u8; 8] = raw[..]
		.try_into()
		.context("gasoline wake deadline is not 8 bytes")?;
	let deadline_ts = i64::from_be_bytes(bytes);

	let workflow_name = read_workflow_name(tx, workflow_id, isolation).await?;
	let pending = tx
		.get(
			&mkeys::deadline_wake_row(&workflow_name, workflow_id, deadline_ts),
			isolation,
		)
		.await?
		.is_some();
	if pending {
		Ok(Some(deadline_ts))
	} else {
		Ok(None)
	}
}

pub async fn read_workflow_name(
	tx: &universaldb::Transaction,
	workflow_id: Id,
	isolation: IsolationLevel,
) -> Result<String> {
	let raw = tx
		.get(&mkeys::workflow_name(workflow_id), isolation)
		.await?
		.context("actor workflow has no name")?;
	String::from_utf8(raw.to_vec()).context("actor workflow name is not utf-8")
}

/// Whether the actor's SQLite is still on the KV-backed v1 format.
///
/// It is when the v1 import marker is set, or when v1 data exists and no v2 head was ever written.
/// v1 data is never deleted after an import, so v1 data plus a v2 head and no marker is a finished
/// import.
pub async fn sqlite_v1(
	tx: &universaldb::Transaction,
	namespace_id: Id,
	actor_id: Id,
	isolation: IsolationLevel,
) -> Result<bool> {
	let pb = tx.with_subspace(keys::subspace());
	let marker = pb
		.exists(&keys::actor::SqliteMigrationKey::new(actor_id), isolation)
		.await?;
	if marker {
		return Ok(true);
	}

	let kv_subspace = keys::actor_kv::subspace(actor_id);
	let mut start = kv_subspace.pack(&keys::actor_kv::ListKeyWrapper(vec![SQLITE_V1_PREFIX]));
	if let Some(&0) = start.last() {
		start.pop();
	}
	let mut end = start.clone();
	end.push(0xFF);

	let mut stream = tx.get_ranges_keyvalues(
		universaldb::RangeOption {
			limit: Some(1),
			mode: StreamingMode::Small,
			..(start, end).into()
		},
		isolation,
	);
	let has_v1_data = stream.try_next().await?.is_some();
	drop(stream);
	if !has_v1_data {
		return Ok(false);
	}

	Ok(read_sqlite_head(tx, namespace_id, actor_id, isolation)
		.await?
		.is_none())
}

/// The actor's published SQLite head, if it has a v2 database.
pub async fn read_sqlite_head(
	tx: &universaldb::Transaction,
	namespace_id: Id,
	actor_id: Id,
	isolation: IsolationLevel,
) -> Result<Option<depot::types::DBHead>> {
	let database_id = actor_id.to_string();
	let bucket_id = BucketId::from_gas_id(namespace_id);

	let key = match depot_branch::resolve_database_branch(tx, bucket_id, &database_id, isolation)
		.await?
	{
		Some(branch_id) => depot_keys::branch_meta_head_key(branch_id),
		None => depot_keys::meta_head_key(&database_id),
	};

	tx.informal()
		.get(&key, isolation)
		.await?
		.map(|bytes| depot::types::decode_db_head(bytes.as_ref()))
		.transpose()
		.context("decode sqlite db head")
}

pub async fn is_parked(tx: &universaldb::Transaction, actor_id: Id) -> Result<bool> {
	Ok(tx
		.get(&mkeys::parked(actor_id), IsolationLevel::Serializable)
		.await?
		.is_some())
}

/// Decodes a persisted parked record.
pub fn decode_parked(raw: &[u8]) -> Result<v2_migration_protocol::ParkedRecord> {
	v2_migration_protocol::versioned::ParkedRecord::deserialize_with_embedded_version(raw)
		.context("decode parked record")
}

/// Encodes a parked record. A record with no delete in progress is written as v1, which every
/// V2 binary reads, so a rollout that adds deletes never leaves an older reader with a record it
/// cannot decode until a delete is first requested.
pub fn encode_parked(record: v2_migration_protocol::ParkedRecord) -> Result<Vec<u8>> {
	let version = if record.delete_requested_at.is_some() || record.deleted_at.is_some() {
		2
	} else {
		1
	};
	v2_migration_protocol::versioned::ParkedRecord::wrap_latest(record)
		.serialize_with_embedded_version(version)
		.context("encode parked record")
}
