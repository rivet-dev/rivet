use std::{collections::BTreeSet, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};
use vbare::OwnedVersionedData;

use crate::{
	cold_tier::ColdTier,
	compactor::SqliteColdCompactPayload,
	conveyer::{
		keys,
		types::{
			DatabaseBranchId, DatabaseBranchRecord, BookmarkStr, CommitRow, MetaCompact,
			SQLITE_STORAGE_COLD_SCHEMA_VERSION, decode_database_branch_record, decode_commit_row,
			decode_meta_compact,
		},
	},
};

use super::worker::ColdCompactorConfig;

pub const SQLITE_COLD_COMPACT_STATE_VERSION: u16 = 1;
pub const SQLITE_COLD_PENDING_MARKER_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdCompactState {
	pub cold_drained_txid: u64,
	pub in_flight_uuid: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColdPendingMarker {
	pub schema_version: u32,
	pub branch_id: DatabaseBranchId,
	pub pass_uuid: uuid::Uuid,
	pub created_at_ms: i64,
	pub cold_drained_txid: u64,
	pub materialized_txid: u64,
	pub last_hot_pass_txid: u64,
	pub planned_object_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdPhaseAPlan {
	pub branch_id: DatabaseBranchId,
	pub pass_uuid: uuid::Uuid,
	pub pending_marker_key: String,
	pub marker: ColdPendingMarker,
	pub state_before: ColdCompactState,
	pub materialized_txid: u64,
	pub last_hot_pass_txid: u64,
	pub branch_record: Option<DatabaseBranchRecord>,
	pub shard_versions: Vec<ColdShardVersion>,
	pub delta_chunks: Vec<ColdDeltaChunk>,
	pub commit_rows: Vec<ColdCommitRow>,
	pub vtx_rows: Vec<ColdVtxRow>,
	pub pin_uploads: Vec<ColdPinUpload>,
	pub database_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdShardVersion {
	pub shard_id: u32,
	pub as_of_txid: u64,
	pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdDeltaChunk {
	pub txid: u64,
	pub chunk_idx: u32,
	pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdCommitRow {
	pub txid: u64,
	pub row: CommitRow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdVtxRow {
	pub versionstamp: [u8; 16],
	pub txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdPinUpload {
	pub database_id: String,
	pub bookmark: BookmarkStr,
	pub versionstamp: [u8; 16],
}

enum VersionedColdCompactState {
	V1(ColdCompactState),
}

impl OwnedVersionedData for VersionedColdCompactState {
	type Latest = ColdCompactState;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite cold compact state version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

enum VersionedColdPendingMarker {
	V1(ColdPendingMarker),
}

impl OwnedVersionedData for VersionedColdPendingMarker {
	type Latest = ColdPendingMarker;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite cold pending marker version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_cold_compact_state(state: ColdCompactState) -> Result<Vec<u8>> {
	VersionedColdCompactState::wrap_latest(state)
		.serialize_with_embedded_version(SQLITE_COLD_COMPACT_STATE_VERSION)
		.context("encode sqlite cold compact state")
}

pub fn decode_cold_compact_state(payload: &[u8]) -> Result<ColdCompactState> {
	VersionedColdCompactState::deserialize_with_embedded_version(payload)
		.context("decode sqlite cold compact state")
}

pub fn encode_pending_marker(marker: ColdPendingMarker) -> Result<Vec<u8>> {
	VersionedColdPendingMarker::wrap_latest(marker)
		.serialize_with_embedded_version(SQLITE_COLD_PENDING_MARKER_VERSION)
		.context("encode sqlite cold pending marker")
}

pub fn decode_pending_marker(payload: &[u8]) -> Result<ColdPendingMarker> {
	VersionedColdPendingMarker::deserialize_with_embedded_version(payload)
		.context("decode sqlite cold pending marker")
}

pub(crate) async fn run(
	db: &universaldb::Database,
	cold_tier: Arc<dyn ColdTier>,
	payload: SqliteColdCompactPayload,
	cold_config: &ColdCompactorConfig,
	cancel_token: tokio_util::sync::CancellationToken,
	now_ms: i64,
) -> Result<ColdPhaseAPlan> {
	ensure_not_cancelled(&cancel_token)?;

	let branch_id = payload_branch_id(&payload);
	let candidate_pass_uuid = uuid::Uuid::new_v4();
	let handoff = register_pending_handoff(db, branch_id, candidate_pass_uuid).await?;
	let pass_uuid = handoff
		.state
		.in_flight_uuid
		.context("sqlite cold phase A handoff did not record pass uuid")?;
	let marker_key = pending_marker_key(branch_id, pass_uuid);
	let pin_uploads = payload_pin_uploads(&payload);
	let database_id = payload_database_id(&payload);
	let marker = ColdPendingMarker {
		schema_version: SQLITE_STORAGE_COLD_SCHEMA_VERSION,
		branch_id,
		pass_uuid,
		created_at_ms: now_ms,
		cold_drained_txid: handoff.state.cold_drained_txid,
		materialized_txid: handoff.materialized_txid,
		last_hot_pass_txid: handoff.last_hot_pass_txid,
		planned_object_keys: initial_planned_object_keys(branch_id, pass_uuid, &payload, &handoff),
	};

	ensure_not_cancelled(&cancel_token)?;
	cold_tier
		.put_object(&marker_key, &encode_pending_marker(marker.clone())?)
		.await
		.with_context(|| format!("put sqlite cold pending marker {marker_key}"))?;

	ensure_not_cancelled(&cancel_token)?;
	let read_timeout = Duration::from_millis(cold_config.phase_a_read_timeout_ms);
	let read_plan = tokio::time::timeout(
		read_timeout,
		read_snapshot_plan(
			db,
			branch_id,
			pass_uuid,
			marker_key.clone(),
			marker.clone(),
			pin_uploads,
			database_id,
		),
	)
	.await
	.context("sqlite cold phase A snapshot read exceeded tx-age budget")??;

	Ok(ColdPhaseAPlan {
		state_before: handoff.state,
		..read_plan
	})
}

async fn register_pending_handoff(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	candidate_pass_uuid: uuid::Uuid,
) -> Result<ColdPhaseAHandoff> {
	db.run(move |tx| async move {
		let state = read_cold_state(&tx, branch_id, Serializable).await?;
		let materialized_txid = read_meta_compact(&tx, branch_id, Serializable)
			.await?
			.map(|compact| compact.materialized_txid)
			.unwrap_or_default();
		let last_hot_pass_txid = read_u64_be(
			&tx,
			&keys::branch_manifest_last_hot_pass_txid_key(branch_id),
			Serializable,
		)
		.await?
		.unwrap_or(materialized_txid);
		let cold_drained_txid = state.cold_drained_txid.max(
			read_u64_be(
				&tx,
				&keys::branch_manifest_cold_drained_txid_key(branch_id),
				Serializable,
			)
			.await?
			.unwrap_or_default(),
		);
		let pass_uuid = state.in_flight_uuid.unwrap_or(candidate_pass_uuid);
		let state = ColdCompactState {
			cold_drained_txid,
			in_flight_uuid: Some(pass_uuid),
		};

		tx.informal().set(
			&keys::branch_meta_cold_compact_key(branch_id),
			&encode_cold_compact_state(state.clone())?,
		);

		Ok(ColdPhaseAHandoff {
			state,
			materialized_txid,
			last_hot_pass_txid,
		})
	})
	.await
}

async fn read_snapshot_plan(
	db: &universaldb::Database,
	branch_id: DatabaseBranchId,
	pass_uuid: uuid::Uuid,
	pending_marker_key: String,
	marker: ColdPendingMarker,
	pin_uploads: Vec<ColdPinUpload>,
	database_id: Option<String>,
) -> Result<ColdPhaseAPlan> {
	db.run(move |tx| {
		let marker = marker.clone();
		let pending_marker_key = pending_marker_key.clone();
		let pin_uploads = pin_uploads.clone();
		let database_id = database_id.clone();
		async move {
			let branch_record = tx
				.informal()
				.get(&keys::branches_list_key(branch_id), Snapshot)
				.await?
				.as_deref()
				.map(|bytes| decode_database_branch_record(bytes))
				.transpose()
				.context("decode sqlite cold phase A branch record")?;
			let materialized_txid = read_meta_compact(&tx, branch_id, Snapshot)
				.await?
				.map(|compact| compact.materialized_txid)
				.unwrap_or_default();
			let last_hot_pass_txid = read_u64_be(
				&tx,
				&keys::branch_manifest_last_hot_pass_txid_key(branch_id),
				Snapshot,
			)
			.await?
			.unwrap_or(materialized_txid);
			let shard_versions = load_shard_versions(&tx, branch_id, marker.cold_drained_txid)
				.await
				.context("load sqlite cold phase A shard versions")?;
			let delta_chunks = load_delta_chunks(&tx, branch_id, marker.cold_drained_txid)
				.await
				.context("load sqlite cold phase A delta chunks")?;
			let commit_rows = load_commit_rows(&tx, branch_id, marker.cold_drained_txid)
				.await
				.context("load sqlite cold phase A commit rows")?;
			let vtx_rows = load_vtx_rows(&tx, branch_id, marker.cold_drained_txid)
				.await
				.context("load sqlite cold phase A vtx rows")?;

			Ok(ColdPhaseAPlan {
				branch_id,
				pass_uuid,
				pending_marker_key,
				marker,
				state_before: ColdCompactState {
					cold_drained_txid: 0,
					in_flight_uuid: None,
				},
				materialized_txid,
				last_hot_pass_txid,
				branch_record,
				shard_versions,
				delta_chunks,
				commit_rows,
				vtx_rows,
				pin_uploads,
				database_id,
			})
		}
	})
	.await
}

async fn read_cold_state(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<ColdCompactState> {
	let Some(bytes) = tx
		.informal()
		.get(&keys::branch_meta_cold_compact_key(branch_id), isolation_level)
		.await?
	else {
		return Ok(ColdCompactState {
			cold_drained_txid: 0,
			in_flight_uuid: None,
		});
	};

	decode_cold_compact_state(&bytes)
}

async fn read_meta_compact(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<MetaCompact>> {
	tx.informal()
		.get(&keys::branch_meta_compact_key(branch_id), isolation_level)
		.await?
		.as_deref()
		.map(|bytes| decode_meta_compact(bytes))
		.transpose()
}

async fn read_u64_be(
	tx: &universaldb::Transaction,
	key: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<u64>> {
	let Some(bytes) = tx.informal().get(key, isolation_level).await? else {
		return Ok(None);
	};
	let bytes: [u8; std::mem::size_of::<u64>()] = bytes
		.as_slice()
		.try_into()
		.context("sqlite cold phase A u64 value had invalid length")?;

	Ok(Some(u64::from_be_bytes(bytes)))
}

async fn load_shard_versions(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cold_drained_txid: u64,
) -> Result<Vec<ColdShardVersion>> {
	let mut out = Vec::new();

	for (key, bytes) in scan_prefix(tx, &keys::branch_shard_prefix(branch_id)).await? {
		let Some((shard_id, as_of_txid)) = decode_branch_shard_version_key(branch_id, &key)? else {
			continue;
		};
		if as_of_txid > cold_drained_txid {
			out.push(ColdShardVersion {
				shard_id,
				as_of_txid,
				bytes,
			});
		}
	}

	Ok(out)
}

async fn load_delta_chunks(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cold_drained_txid: u64,
) -> Result<Vec<ColdDeltaChunk>> {
	let mut out = Vec::new();

	for (key, bytes) in scan_prefix(tx, &keys::branch_delta_prefix(branch_id)).await? {
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, &key)?;
		if txid <= cold_drained_txid {
			continue;
		}
		let chunk_idx = keys::decode_branch_delta_chunk_idx(branch_id, txid, &key)?;
		out.push(ColdDeltaChunk {
			txid,
			chunk_idx,
			bytes,
		});
	}

	Ok(out)
}

async fn load_commit_rows(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cold_drained_txid: u64,
) -> Result<Vec<ColdCommitRow>> {
	let mut out = Vec::new();

	for (key, bytes) in scan_prefix(tx, &keys::branch_commit_prefix(branch_id)).await? {
		let txid = decode_suffix_u64(&keys::branch_commit_prefix(branch_id), &key)
			.context("decode sqlite cold phase A commit txid")?;
		if txid > cold_drained_txid {
			out.push(ColdCommitRow {
				txid,
				row: decode_commit_row(&bytes)?,
			});
		}
	}

	Ok(out)
}

async fn load_vtx_rows(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	cold_drained_txid: u64,
) -> Result<Vec<ColdVtxRow>> {
	let mut out = Vec::new();

	for (key, bytes) in scan_prefix(tx, &keys::branch_vtx_prefix(branch_id)).await? {
		let versionstamp = decode_suffix_versionstamp(&keys::branch_vtx_prefix(branch_id), &key)
			.context("decode sqlite cold phase A vtx versionstamp")?;
		let txid = decode_u64_be_value(&bytes).context("decode sqlite cold phase A vtx txid")?;
		if txid > cold_drained_txid {
			out.push(ColdVtxRow {
				versionstamp,
				txid,
			});
		}
	}

	Ok(out)
}

async fn scan_prefix(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn initial_planned_object_keys(
	branch_id: DatabaseBranchId,
	pass_uuid: uuid::Uuid,
	payload: &SqliteColdCompactPayload,
	handoff: &ColdPhaseAHandoff,
) -> Vec<String> {
	let prefix = branch_object_prefix(branch_id);
	let mut keys = BTreeSet::from([
		format!("{prefix}/branch_record.bare"),
		format!("{prefix}/cold_manifest/index.bare"),
		format!("{prefix}/cold_manifest/chunks/{}.bare", pass_uuid.simple()),
		format!("{prefix}/pointer_snapshot/{}.bare", pass_uuid.simple()),
	]);

	if handoff.materialized_txid > handoff.state.cold_drained_txid {
		keys.insert(format!(
			"{prefix}/delta/{:016x}-{:016x}.ltx",
			handoff.state.cold_drained_txid.saturating_add(1),
			handoff.materialized_txid
		));
	}

	if let SqliteColdCompactPayload::CreatePinnedBookmark { versionstamp, .. } = payload {
		keys.insert(format!("{prefix}/pin/{}.ltx", hex_bytes(versionstamp)));
	}

	keys.into_iter().collect()
}

fn pending_marker_key(branch_id: DatabaseBranchId, pass_uuid: uuid::Uuid) -> String {
	format!(
		"{}/pending/{}.marker",
		branch_object_prefix(branch_id),
		pass_uuid.simple()
	)
}

fn branch_object_prefix(branch_id: DatabaseBranchId) -> String {
	format!("db/{}", branch_id.as_uuid().simple())
}

fn decode_branch_shard_version_key(
	branch_id: DatabaseBranchId,
	key: &[u8],
) -> Result<Option<(u32, u64)>> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch shard key did not start with expected prefix")?;
	if suffix.len() == std::mem::size_of::<u32>() {
		return Ok(None);
	}
	if suffix.len() != std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
		|| suffix[std::mem::size_of::<u32>()] != b'/'
	{
		bail!("branch shard version key suffix had invalid length");
	}
	let shard_id = u32::from_be_bytes(
		suffix[..std::mem::size_of::<u32>()]
			.try_into()
			.context("decode branch shard id")?,
	);
	let as_of_txid = u64::from_be_bytes(
		suffix[std::mem::size_of::<u32>() + 1..]
			.try_into()
			.context("decode branch shard txid")?,
	);

	Ok(Some((shard_id, as_of_txid)))
}

fn decode_suffix_u64(prefix: &[u8], key: &[u8]) -> Result<u64> {
	let suffix = key
		.strip_prefix(prefix)
		.context("key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u64>()] = suffix
		.try_into()
		.context("key suffix had invalid u64 length")?;

	Ok(u64::from_be_bytes(bytes))
}

fn decode_suffix_versionstamp(prefix: &[u8], key: &[u8]) -> Result<[u8; 16]> {
	let suffix = key
		.strip_prefix(prefix)
		.context("key did not start with expected prefix")?;
	let bytes: [u8; 16] = suffix
		.try_into()
		.context("key suffix had invalid versionstamp length")?;

	Ok(bytes)
}

fn decode_u64_be_value(value: &[u8]) -> Result<u64> {
	let bytes: [u8; std::mem::size_of::<u64>()] = value
		.try_into()
		.context("value had invalid u64 length")?;

	Ok(u64::from_be_bytes(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
	bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn payload_branch_id(payload: &SqliteColdCompactPayload) -> DatabaseBranchId {
	match payload {
		SqliteColdCompactPayload::CreatePinnedBookmark {
			database_branch_id, ..
		}
		| SqliteColdCompactPayload::DeletePinnedBookmark {
			database_branch_id, ..
		} => *database_branch_id,
		SqliteColdCompactPayload::ForkWarmup {
			target_database_branch_id,
			..
		} => *target_database_branch_id,
		SqliteColdCompactPayload::NamespaceForkWarmup { .. } => {
			DatabaseBranchId::nil()
		}
	}
}

fn payload_pin_uploads(payload: &SqliteColdCompactPayload) -> Vec<ColdPinUpload> {
	match payload {
		SqliteColdCompactPayload::CreatePinnedBookmark {
			database_id,
			bookmark,
			versionstamp,
			..
		} => vec![ColdPinUpload {
			database_id: database_id.clone(),
			bookmark: bookmark.clone(),
			versionstamp: *versionstamp,
		}],
		SqliteColdCompactPayload::DeletePinnedBookmark { .. } => Vec::new(),
		SqliteColdCompactPayload::ForkWarmup { .. }
		| SqliteColdCompactPayload::NamespaceForkWarmup { .. } => Vec::new(),
	}
}

fn payload_database_id(payload: &SqliteColdCompactPayload) -> Option<String> {
	match payload {
		SqliteColdCompactPayload::CreatePinnedBookmark { database_id, .. }
		| SqliteColdCompactPayload::DeletePinnedBookmark { database_id, .. } => Some(database_id.clone()),
		SqliteColdCompactPayload::ForkWarmup { .. }
		| SqliteColdCompactPayload::NamespaceForkWarmup { .. } => None,
	}
}

fn ensure_not_cancelled(cancel_token: &tokio_util::sync::CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite cold compaction cancelled");
	}

	Ok(())
}

#[derive(Debug)]
struct ColdPhaseAHandoff {
	state: ColdCompactState,
	materialized_txid: u64,
	last_hot_pass_txid: u64,
}
