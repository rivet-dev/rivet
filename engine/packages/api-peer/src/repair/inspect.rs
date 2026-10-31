//! Read and inspect routes. Every one is side-effect free: maintainer inspections, the shards'
//! deliberately unfenced status and entry reads, one-shot row reads and checkpoint listings.

use super::{
	Refusal, RepairResult,
	json::{self, Position, RowKey, RowRange, VolumeKey, VolumeLogKey},
};
use axum::{Json, extract::State};
use depot_volume::ups::UpsLogClient;
use futures_util::future::join_all;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		CheckpointList, CheckpointOperation, ClientRequest, ClientResponse, RepairInspectRequest,
		RepairInspection, VolumeCheckpointRequest,
	},
	depot_log_shard_v1::{VolumeLogEntryBody, VolumeLogReplicaState},
	depot_storage_worker_protocol_v1::{
		ReadVolumeLogEntriesRequest, ReadVolumeLogStatusRequest, StorageWorkerRequest,
		StorageWorkerResponse, VolumeLogStatus,
	},
	depot_tablet_manifest_v1::Tablet,
	shared_depot_fs_read_v1::{
		FsReadAt, FsReadClose, FsReadError, FsReadOpen, FsReadOperation, FsReadRaw, FsReadRequest,
		FsReadResponse, FsReadScan, FsReadTarget, FsSessionRead,
	},
	shared_depot_types_v1::{self as wire, VolumeLogReplicaStatus},
};
use rivet_engine_context::EngineContext;
use rivet_topology::worker::WorkerId;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use universalpubsub::SubjectCapacities;
use vbare::Versioned;

/// Most rows one scan returns.
const MAX_SCAN_ROWS: u32 = 1000;

/// Most bytes one log read returns.
const MAX_LOG_READ_BYTES: u32 = 1024 * 1024;

/// Most row versions one raw read returns.
const MAX_ROW_VERSIONS: u32 = 64;

/// Most checkpoints one listing returns.
const MAX_CHECKPOINTS: u32 = 1000;

/// How long a caller waits before retrying a request this worker could not route or send.
const ROUTE_RETRY_MS: u64 = 100;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct VolumeBody {
	volume_key: VolumeKey,
}

/// Names a volume and, optionally, one of its incarnations. Without one, the routes use the
/// incarnation the maintainer would serve, or the name's pointer when nothing is loaded.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct IncarnationBody {
	volume_key: VolumeKey,
	#[serde(default)]
	volume_incarnation: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LogReadBody {
	volume_log_key: VolumeLogKey,
	worker: String,
	first_volume_seq: u64,
	#[serde(default)]
	max_bytes: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RowsGetBody {
	volume_key: VolumeKey,
	#[serde(default)]
	volume_incarnation: Option<String>,
	/// An exact position to read at; latest applied when absent.
	#[serde(default)]
	position: Option<Position>,
	key: RowKey,
	#[serde(default)]
	versions: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RowsScanBody {
	volume_key: VolumeKey,
	#[serde(default)]
	volume_incarnation: Option<String>,
	#[serde(default)]
	position: Option<Position>,
	range: RowRange,
	#[serde(default)]
	after: Option<RowKey>,
	#[serde(default)]
	limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CheckpointsBody {
	volume_key: VolumeKey,
	#[serde(default)]
	after: Option<String>,
	#[serde(default)]
	limit: Option<u32>,
}

pub(crate) async fn volume_inspect(
	State(ctx): State<EngineContext>,
	Json(body): Json<VolumeBody>,
) -> RepairResult {
	let inspection = inspect(&ctx, body.volume_key.wire()?).await?;
	Ok(Json(json::wire(&inspection)))
}

pub(crate) async fn placement(
	State(ctx): State<EngineContext>,
	Json(body): Json<VolumeBody>,
) -> RepairResult {
	let key = body.volume_key.wire()?;
	let tablet = placement_of(&ctx, &key)?;
	let inspection = inspect(&ctx, key.clone()).await?;
	Ok(Json(json!({
		"partitionKeyHash": hex::encode(depot_types::partition_key_hash(&key)),
		"manifest": json::wire(&tablet),
		"maintainer": {
			"tabletId": hex::encode(inspection.tablet_id),
			"tabletGrantEpoch": inspection.tablet_grant_epoch,
			"replicaSet": json::wire(&inspection.replica_set),
		},
		"agree": inspection.tablet_id == tablet.id
			&& inspection.tablet_grant_epoch == tablet.tablet_grant_epoch
			&& inspection.replica_set == tablet.replica_set,
	})))
}

pub(crate) async fn replicas(
	State(ctx): State<EngineContext>,
	Json(body): Json<IncarnationBody>,
) -> RepairResult {
	let key = body.volume_key.wire()?;
	let inspection = inspect(&ctx, key.clone()).await?;
	let log_key = incarnation(&inspection, &key, body.volume_incarnation.as_deref())?;
	let tablet = placement_of(&ctx, &key)?;
	let statuses = shard_statuses(&ctx, &tablet, &log_key).await?;
	Ok(Json(json!({
		"volumeLogKey": json::wire(&log_key),
		"replicas": statuses
			.iter()
			.map(|(worker, status)| json!({
				"worker": hex::encode(worker.as_bytes()),
				"status": match status {
					Ok(status) => json::wire(status),
					Err(error) => json!({ "Unreachable": error }),
				},
			}))
			.collect::<Vec<_>>(),
	})))
}

pub(crate) async fn diff(
	State(ctx): State<EngineContext>,
	Json(body): Json<IncarnationBody>,
) -> RepairResult {
	let key = body.volume_key.wire()?;
	let inspection = inspect(&ctx, key.clone()).await?;
	let log_key = incarnation(&inspection, &key, body.volume_incarnation.as_deref())?;
	let tablet = placement_of(&ctx, &key)?;
	let statuses = shard_statuses(&ctx, &tablet, &log_key).await?;
	Ok(Json(compare(&inspection, &tablet, &log_key, &statuses)))
}

pub(crate) async fn log_read(
	State(ctx): State<EngineContext>,
	Json(body): Json<LogReadBody>,
) -> RepairResult {
	let log_key = body.volume_log_key.wire()?;
	let worker = WorkerId::from_bytes(json::uuid(&body.worker, "worker")?);
	let max_bytes = body.max_bytes.unwrap_or(MAX_LOG_READ_BYTES);
	if max_bytes == 0 || max_bytes > MAX_LOG_READ_BYTES {
		return Err(Refusal::invalid(format!(
			"maxBytes must be from 1 to {MAX_LOG_READ_BYTES}"
		)));
	}
	let response = log_client(&ctx)?
		.request(
			&worker,
			StorageWorkerRequest::ReadVolumeLogEntriesRequest(ReadVolumeLogEntriesRequest {
				volume_log_key: log_key.clone(),
				first_volume_seq: body.first_volume_seq,
				max_bytes,
				include_deleted: false,
			}),
		)
		.await
		.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))?;
	match response {
		StorageWorkerResponse::ReadVolumeLogEntriesResponse(response) => Ok(Json(json!({
			"volumeLogKey": json::wire(&log_key),
			"worker": hex::encode(worker.as_bytes()),
			"state": json::wire(&response.state),
			"entries": response.entries.iter().map(entry_json).collect::<Vec<_>>(),
		}))),
		StorageWorkerResponse::RejectResponse(reject) => Ok(Json(json!({
			"volumeLogKey": json::wire(&log_key),
			"worker": hex::encode(worker.as_bytes()),
			"rejected": json::wire(&reject),
		}))),
		other => Err(Refusal::unavailable(
			format!("the shard answered a log read with {other:?}"),
			ROUTE_RETRY_MS,
		)),
	}
}

pub(crate) async fn rows_get(
	State(ctx): State<EngineContext>,
	Json(body): Json<RowsGetBody>,
) -> RepairResult {
	let versions = body.versions.unwrap_or(4);
	if versions == 0 || versions > MAX_ROW_VERSIONS {
		return Err(Refusal::invalid(format!(
			"versions must be from 1 to {MAX_ROW_VERSIONS}"
		)));
	}
	let target = read_target(&body.volume_key, body.volume_incarnation.as_deref())?;
	let response = read(
		&ctx,
		FsReadRequest {
			target,
			client_epoch: None,
			operation: FsReadOperation::FsReadRaw(FsReadRaw {
				position: body.position.as_ref().map(Position::wire),
				key: body.key.wire()?,
				versions,
			}),
		},
	)
	.await?;
	Ok(Json(json::wire(&response)))
}

pub(crate) async fn rows_scan(
	State(ctx): State<EngineContext>,
	Json(body): Json<RowsScanBody>,
) -> RepairResult {
	let limit = body.limit.unwrap_or(MAX_SCAN_ROWS);
	if limit == 0 || limit > MAX_SCAN_ROWS {
		return Err(Refusal::invalid(format!(
			"limit must be from 1 to {MAX_SCAN_ROWS}"
		)));
	}
	let range = body.range.wire()?;
	let after = body.after.as_ref().map(RowKey::wire).transpose()?;
	let target = read_target(&body.volume_key, body.volume_incarnation.as_deref())?;
	let opened = read(
		&ctx,
		FsReadRequest {
			target,
			client_epoch: None,
			operation: FsReadOperation::FsReadOpen(FsReadOpen {
				position: body.position.as_ref().map(Position::wire),
				prefetch: None,
			}),
		},
	)
	.await?;
	let FsReadResponse::FsReadOpened(opened) = opened else {
		return Err(Refusal::unavailable(
			"the open answered no session",
			ROUTE_RETRY_MS,
		));
	};
	let Some(session) = opened.session.as_ref().map(|session| session.id) else {
		return Err(Refusal::unavailable(
			"the open answered no session",
			ROUTE_RETRY_MS,
		));
	};
	let scanned = read(
		&ctx,
		FsReadRequest {
			target: FsReadTarget::VolumeLogKey(opened.volume_log_key.clone()),
			client_epoch: None,
			operation: FsReadOperation::FsReadScan(FsReadScan {
				at: FsReadAt::FsSessionRead(FsSessionRead { session }),
				range,
				after,
				limit,
			}),
		},
	)
	.await;
	// The session is released whatever the scan answered; its lifetime bounds it otherwise.
	let _ = read(
		&ctx,
		FsReadRequest {
			target: FsReadTarget::VolumeLogKey(opened.volume_log_key.clone()),
			client_epoch: None,
			operation: FsReadOperation::FsReadClose(FsReadClose { session }),
		},
	)
	.await;
	Ok(Json(json!({
		"volumeLogKey": json::wire(&opened.volume_log_key),
		"applied": json::wire(&opened.applied),
		"asOf": opened.as_of,
		"rows": json::wire(&scanned?),
	})))
}

pub(crate) async fn checkpoints(
	State(ctx): State<EngineContext>,
	Json(body): Json<CheckpointsBody>,
) -> RepairResult {
	let limit = body.limit.unwrap_or(MAX_CHECKPOINTS);
	if limit == 0 || limit > MAX_CHECKPOINTS {
		return Err(Refusal::invalid(format!(
			"limit must be from 1 to {MAX_CHECKPOINTS}"
		)));
	}
	let response = maintainer(
		&ctx,
		ClientRequest::VolumeCheckpointRequest(VolumeCheckpointRequest {
			volume_key: body.volume_key.wire()?,
			operation: CheckpointOperation::CheckpointList(CheckpointList {
				after: body
					.after
					.as_deref()
					.map(|after| json::unhex(after, "after"))
					.transpose()?
					.map(Into::into),
				limit,
			}),
		}),
	)
	.await?;
	match response {
		ClientResponse::VolumeCheckpointResponse(response) => Ok(Json(json::wire(&response))),
		other => Err(unexpected(other)),
	}
}

/// The maintainer's inspection of `key`.
pub(crate) async fn inspect(
	ctx: &EngineContext,
	key: wire::VolumeKey,
) -> Result<RepairInspection, Refusal> {
	match maintainer(
		ctx,
		ClientRequest::RepairInspectRequest(RepairInspectRequest { volume_key: key }),
	)
	.await?
	{
		ClientResponse::RepairInspection(inspection) => Ok(inspection),
		other => Err(unexpected(other)),
	}
}

/// Sends one repair request to the maintainer of the volume it names.
pub(crate) async fn maintainer(
	ctx: &EngineContext,
	request: ClientRequest,
) -> Result<ClientResponse, Refusal> {
	let client = ctx.depot();
	let deadline = deadline(ctx)?;
	let response = client
		.repair_request(request, deadline)
		.await
		.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))?;
	match response {
		ClientResponse::VolumeRejectResponse(reason) => Err(Refusal::rejected(reason)),
		response => Ok(response),
	}
}

fn deadline(ctx: &EngineContext) -> Result<u64, Refusal> {
	let clock = ctx.env().clock();
	rivet_env::deadline_after(
		clock.as_ref(),
		Duration::from_millis(ctx.config().depot().volume_request_timeout_ms()),
	)
	.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))
}

fn unexpected(response: ClientResponse) -> Refusal {
	Refusal::unavailable(
		format!("the maintainer answered with {response:?}"),
		ROUTE_RETRY_MS,
	)
}

/// Reads through the maintainer's read admission, mapping read errors to refusals.
async fn read(ctx: &EngineContext, request: FsReadRequest) -> Result<FsReadResponse, Refusal> {
	let client = ctx.depot();
	let deadline = deadline(ctx)?;
	let response = client
		.read_filesystem(request, deadline)
		.await
		.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))?;
	match response {
		FsReadResponse::FsReadError(error) => Err(read_refusal(error)),
		response => Ok(response),
	}
}

fn read_refusal(error: FsReadError) -> Refusal {
	let (status, code, retry_after_ms) = match error {
		FsReadError::NotFound => (http::StatusCode::NOT_FOUND, "NOT_FOUND", None),
		FsReadError::TooLarge => (http::StatusCode::PAYLOAD_TOO_LARGE, "TOO_LARGE", None),
		FsReadError::Corrupt => (http::StatusCode::CONFLICT, "CORRUPT", None),
		FsReadError::TooOld => (http::StatusCode::CONFLICT, "TOO_OLD", None),
		FsReadError::Lagging => (
			http::StatusCode::SERVICE_UNAVAILABLE,
			"LAGGING",
			Some(ROUTE_RETRY_MS),
		),
		FsReadError::Overloaded => (
			http::StatusCode::SERVICE_UNAVAILABLE,
			"OVERLOADED",
			Some(ROUTE_RETRY_MS),
		),
		FsReadError::BaseUnavailable => (
			http::StatusCode::SERVICE_UNAVAILABLE,
			"BASE_UNAVAILABLE",
			Some(ROUTE_RETRY_MS),
		),
		FsReadError::Fenced | FsReadError::Unavailable | FsReadError::NotServing => (
			http::StatusCode::SERVICE_UNAVAILABLE,
			"UNAVAILABLE",
			Some(ROUTE_RETRY_MS),
		),
	};
	Refusal {
		status,
		code,
		message: format!("{error:?}"),
		retry_after_ms,
		current: None,
	}
}

fn read_target(key: &VolumeKey, incarnation: Option<&str>) -> Result<FsReadTarget, Refusal> {
	let volume_key = key.wire()?;
	Ok(match incarnation {
		Some(incarnation) => FsReadTarget::VolumeLogKey(wire::VolumeLogKey {
			volume_key,
			volume_incarnation: json::uuid(incarnation, "volumeIncarnation")?,
		}),
		None => FsReadTarget::VolumeKey(volume_key),
	})
}

fn placement_of(ctx: &EngineContext, key: &wire::VolumeKey) -> Result<Tablet, Refusal> {
	ctx.depot()
		.placement(key)
		.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))
}

/// The incarnation a route acts on: the named one, or else the newest loaded incarnation that
/// is not being deleted, or else the incarnation the name's pointer names.
fn incarnation(
	inspection: &RepairInspection,
	key: &wire::VolumeKey,
	named: Option<&str>,
) -> Result<wire::VolumeLogKey, Refusal> {
	if let Some(named) = named {
		return Ok(wire::VolumeLogKey {
			volume_key: key.clone(),
			volume_incarnation: json::uuid(named, "volumeIncarnation")?,
		});
	}
	let loaded = inspection
		.incarnations
		.iter()
		.filter(|view| !view.deleting)
		.max_by_key(|view| {
			view.known
				.as_ref()
				.or(view.evidence.as_ref())
				.map_or(0, |record| record.incarnation_epoch)
		})
		.map(|view| view.volume_log_key.clone());
	let pointer = inspection
		.pointer
		.as_ref()
		.map(|record| wire::VolumeLogKey {
			volume_key: key.clone(),
			volume_incarnation: record.volume_incarnation,
		});
	loaded
		.or(pointer)
		.ok_or_else(|| Refusal::not_found("no incarnation of the name is loaded or stored"))
}

fn log_client(ctx: &EngineContext) -> Result<UpsLogClient, Refusal> {
	let settings = ctx.config().depot();
	UpsLogClient::new(
		ctx.ups()
			.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))?,
		SubjectCapacities {
			incoming: settings.log_request_capacity(),
			outgoing: settings.log_request_capacity(),
		},
		Duration::from_millis(settings.volume_request_timeout_ms()),
		Arc::new(|| true),
	)
	.map_err(|error| Refusal::unavailable(format!("{error:#}"), ROUTE_RETRY_MS))
}

/// Every worker that may hold a replica of the tablet's volumes: its members and the workers
/// of its disks, which include a learner.
fn destinations(tablet: &Tablet) -> BTreeSet<[u8; 16]> {
	tablet
		.replica_set
		.members
		.iter()
		.copied()
		.chain(tablet.replica_set.disks.iter().map(|disk| disk.worker_id))
		.collect()
}

/// Each destination's status of `key`, read with the shards' unfenced status read.
async fn shard_statuses(
	ctx: &EngineContext,
	tablet: &Tablet,
	key: &wire::VolumeLogKey,
) -> Result<Vec<(WorkerId, Result<VolumeLogStatus, String>)>, Refusal> {
	let client = log_client(ctx)?;
	let reads = destinations(tablet).into_iter().map(|worker| {
		let client = client.clone();
		let key = key.clone();
		async move {
			let worker = WorkerId::from_bytes(worker);
			let result = client
				.request(
					&worker,
					StorageWorkerRequest::ReadVolumeLogStatusRequest(ReadVolumeLogStatusRequest {
						volume_log_keys: vec![key],
					}),
				)
				.await;
			let status = match result {
				Ok(StorageWorkerResponse::ReadVolumeLogStatusResponse(mut response)) => {
					match response.statuses.pop() {
						Some(status) => Ok(status),
						None => Err("the shard answered no status".to_owned()),
					}
				}
				Ok(other) => Err(format!("{other:?}")),
				Err(error) => Err(format!("{error:#}")),
			};
			(worker, status)
		}
	});
	Ok(join_all(reads).await)
}

/// The replica state a status carries, when the shard holds the replica.
fn status_state(status: &VolumeLogStatus) -> Option<&VolumeLogReplicaState> {
	match status {
		VolumeLogStatus::BelowCold(status) => Some(&status.state),
		VolumeLogStatus::VolumeLogReplicaDeleted(status) => Some(&status.state),
		VolumeLogStatus::VolumeNotLoaded | VolumeLogStatus::NotServing => None,
	}
}

/// Compares the maintainer's view of every member with what its shard reports, and names
/// each disagreement with a stable verdict:
///
/// `UNREACHABLE` (no status), `NOT_LOADED`, `DELETED`, `LOST_ENTRIES`, `REPAIRING`, `EXTRA` (a
/// replica outside the replica set), `COLD_MISMATCH` and `CLAIM_MISMATCH` (the shard and the
/// maintainer's view disagree on cold or the claim), `AHEAD` and `BEHIND` (the shard's
/// appended position is past or short of the maintainer's view), `DIVERGED` (the shard holds
/// another epoch at a sequence an in-sync member also holds), and `AGREE`.
fn compare(
	inspection: &RepairInspection,
	tablet: &Tablet,
	key: &wire::VolumeLogKey,
	statuses: &[(WorkerId, Result<VolumeLogStatus, String>)],
) -> Value {
	let view = inspection
		.incarnations
		.iter()
		.find(|view| view.volume_log_key == *key);
	let believed = |worker: &WorkerId| {
		view.and_then(|view| {
			view.members
				.iter()
				.find(|member| member.worker_id == *worker.as_bytes())
				.map(|member| &member.state)
		})
	};
	let in_sync_tails = statuses
		.iter()
		.filter_map(|(_, status)| status.as_ref().ok().and_then(status_state))
		.filter(|state| state.status == VolumeLogReplicaStatus::InSync && !state.lost_entries)
		.map(|state| state.appended.clone())
		.collect::<Vec<_>>();
	let members = tablet
		.replica_set
		.members
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();

	let mut in_sync = 0usize;
	let rows = statuses
		.iter()
		.map(|(worker, status)| {
			let state = status.as_ref().ok().and_then(status_state);
			let verdict = match status {
				Err(_) => "UNREACHABLE",
				Ok(VolumeLogStatus::VolumeNotLoaded | VolumeLogStatus::NotServing) => "NOT_LOADED",
				Ok(VolumeLogStatus::VolumeLogReplicaDeleted(_)) => "DELETED",
				Ok(VolumeLogStatus::BelowCold(status)) => verdict(
					&status.state,
					believed(worker),
					&in_sync_tails,
					members.contains(worker.as_bytes()),
				),
			};
			if members.contains(worker.as_bytes())
				&& matches!(
					verdict,
					"AGREE" | "AHEAD" | "BEHIND" | "COLD_MISMATCH" | "CLAIM_MISMATCH"
				) {
				in_sync += 1;
			}
			json!({
				"worker": hex::encode(worker.as_bytes()),
				"member": members.contains(worker.as_bytes()),
				"verdict": verdict,
				"state": state.map(json::wire),
				"believed": believed(worker).map(json::wire),
				"error": status.as_ref().err(),
			})
		})
		.collect::<Vec<_>>();
	let needed = members.len() / 2 + 1;
	let registered = view.and_then(|view| view.applied.clone());
	let record = view.and_then(|view| view.known.clone().or(view.evidence.clone()));
	json!({
		"volumeLogKey": json::wire(key),
		"maintainer": {
			"loaded": view.is_some(),
			"applied": registered.map(|applied| json::wire(&applied)),
			"cold": record.map(|record| json::wire(&record.cold)),
			"holder": view.and_then(|view| view.holder.as_ref()).map(json::wire),
			"recordMissing": view.is_some_and(|view| view.record_missing),
			"deleting": view.is_some_and(|view| view.deleting),
			"unloading": view.is_some_and(|view| view.unloading),
		},
		"members": rows,
		"quorum": {
			"inSync": in_sync,
			"needed": needed,
			"verdict": if in_sync >= needed { "QUORUM" } else { "NO_QUORUM" },
		},
	})
}

fn verdict(
	state: &VolumeLogReplicaState,
	believed: Option<&VolumeLogReplicaState>,
	in_sync_tails: &[wire::VolumeLogPosition],
	member: bool,
) -> &'static str {
	if !member {
		return "EXTRA";
	}
	if state.lost_entries {
		return "LOST_ENTRIES";
	}
	match state.status {
		VolumeLogReplicaStatus::Repairing => return "REPAIRING",
		VolumeLogReplicaStatus::Deleted => return "DELETED",
		VolumeLogReplicaStatus::InSync => {}
	}
	let diverged = in_sync_tails.iter().any(|tail| {
		tail.volume_seq == state.appended.volume_seq
			&& tail.client_epoch != state.appended.client_epoch
	});
	if diverged {
		return "DIVERGED";
	}
	let Some(believed) = believed else {
		return "AGREE";
	};
	if believed.cold != state.cold {
		return "COLD_MISMATCH";
	}
	if believed.claimed_client != state.claimed_client {
		return "CLAIM_MISMATCH";
	}
	match state.appended.volume_seq.cmp(&believed.appended.volume_seq) {
		std::cmp::Ordering::Greater => "AHEAD",
		std::cmp::Ordering::Less => "BEHIND",
		std::cmp::Ordering::Equal
			if state.appended.client_epoch != believed.appended.client_epoch =>
		{
			"DIVERGED"
		}
		std::cmp::Ordering::Equal => "AGREE",
	}
}

/// One log entry with its payload decoded into filesystem ops when it is a filesystem entry.
fn entry_json(entry: &rivet_data::generated::depot_log_shard_v1::VolumeLogEntry) -> Value {
	let body = match &entry.body {
		VolumeLogEntryBody::ClaimEntry => json!("ClaimEntry"),
		VolumeLogEntryBody::PayloadWrite(write) => {
			let decoded = depot_types::payload_codec::decode(write);
			let (digest_ok, decoded) = match decoded {
				Ok(bytes) => {
					let digest = depot_types::payload_codec::payload_digest(&bytes);
					let entry = (write.payload_family
						== rivet_data::families::VersionFamily::DEPOT_FS_ENTRY.0)
						.then(|| {
							rivet_data::versioned::DepotFsEntryData::deserialize_with_embedded_version(&bytes)
						});
					let decoded = match entry {
						Some(Ok(entry)) => json!({ "entry": json::wire(&entry) }),
						Some(Err(error)) => json!({ "decodeError": format!("{error:#}") }),
						None => json!({ "raw": hex::encode(&bytes) }),
					};
					(Some(digest == write.payload_digest), decoded)
				}
				Err(error) => (None, json!({ "decodeError": format!("{error:#}") })),
			};
			json!({
				"PayloadWrite": {
					"payloadFamily": write.payload_family,
					"payloadVersion": write.payload_version,
					"clientSeq": write.client_seq,
					"requestId": hex::encode(write.request_id),
					"payloadDigest": hex::encode(write.payload_digest),
					"digestMatches": digest_ok,
					"codec": format!("{:?}", write.codec),
					"payloadBytes": write.payload.len(),
					"decoded": decoded,
				}
			})
		}
	};
	json!({
		"volumeSeq": entry.volume_seq,
		"clientEpoch": entry.client_epoch,
		"writtenAtUnixMs": entry.written_at_unix_ms,
		"body": body,
	})
}


