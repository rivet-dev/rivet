//! Mutating repair routes. Each takes the common envelope, defaults to a dry run, and answers
//! in one shape:
//!
//! ```jsonc
//! { "operationId": "…", "result": "PLANNED" | "APPLIED" | "ALREADY_APPLIED" | "REFUSED" | "UNKNOWN_OUTCOME",
//!   "plan": { }, "planDigest": "…", "refusal": { "code": "…", "retryAfterMs": null, "current": { } } }
//! ```
//!
//! `UNKNOWN_OUTCOME` means the effect may have run, for example when the reply was lost after
//! the call's intent was logged. Resending the same `operationId` answers from the audit rows.

use super::{
	Refusal, RepairResult,
	inspect::maintainer,
	json::{self, Position, VolumeKey, VolumeLogKey},
};
use axum::{Json, extract::State};
use http::StatusCode;
use rivet_data::generated::{
	depot_client_protocol_v1::{
		ClientRequest, ClientResponse, RepairAppend, RepairAuditListRequest, RepairBody,
		RepairClaimAcquire, RepairClaimRelease, RepairEnvelope, RepairPreimage,
		RepairRebuildRecord, RepairReplicaForget, RepairReplicaReload, RepairRequest,
		RepairResponse,
	},
	depot_fs_entry_v1::{FsEntry, FsTxn, FsTxnAbort},
	shared_depot_fs_v1::{FsKvDelete, FsKvDeleteRange, FsKvPut, FsOp},
	shared_depot_types_v1::ClientIdentity,
};
use rivet_engine_context::EngineContext;
use serde::Deserialize;
use serde_json::{Value, json};

/// The credential the repair routes record as the principal of a call. Only the admin token
/// authorizes these routes, so every call is the admin's.
const ADMIN_PRINCIPAL: &str = "admin-token";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Fence {
	volume_log_key: VolumeLogKey,
	tablet: TabletFence,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TabletFence {
	tablet_id: String,
	tablet_grant_epoch: u64,
}

/// The envelope of every mutating call, with the body type of its route.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Envelope<B> {
	operation_id: String,
	#[serde(default = "dry_run_default")]
	dry_run: bool,
	reason: String,
	fence: Fence,
	#[serde(default)]
	plan_digest: Option<String>,
	body: B,
}

fn dry_run_default() -> bool {
	true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RebuildBody {
	expected_cold: Position,
	#[serde(default)]
	allow_missing_evidence: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AcquireBody {
	ttl_ms: u64,
	#[serde(default)]
	fence_holder: bool,
	#[serde(default)]
	expected_appended: Option<Position>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Claim {
	client_epoch: u64,
	client_incarnation: String,
}

impl Claim {
	fn wire(&self) -> Result<ClientIdentity, Refusal> {
		Ok(ClientIdentity {
			client_epoch: self.client_epoch,
			client_incarnation: json::uuid(&self.client_incarnation, "clientIncarnation")?,
		})
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReleaseBody {
	claim: Claim,
}

/// One op of a corrective entry. An append holds only KV ops.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum Op {
	FsKvPut {
		ino: u64,
		key: String,
		value: String,
	},
	FsKvDelete {
		ino: u64,
		key: String,
	},
	FsKvDeleteRange {
		ino: u64,
		start: String,
		#[serde(default)]
		end: Option<String>,
	},
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum Txn {
	FsTxnAbort { id: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Entry {
	#[serde(default)]
	ops: Vec<Op>,
	#[serde(default)]
	txn: Option<Txn>,
}

impl Entry {
	fn wire(&self) -> Result<FsEntry, Refusal> {
		let ops = self
			.ops
			.iter()
			.map(|op| {
				Ok(match op {
					Op::FsKvPut { ino, key, value } => FsOp::FsKvPut(FsKvPut {
						ino: *ino,
						key: json::unhex(key, "key")?.into(),
						value: json::unhex(value, "value")?.into(),
					}),
					Op::FsKvDelete { ino, key } => FsOp::FsKvDelete(FsKvDelete {
						ino: *ino,
						key: json::unhex(key, "key")?.into(),
					}),
					Op::FsKvDeleteRange { ino, start, end } => {
						FsOp::FsKvDeleteRange(FsKvDeleteRange {
							ino: *ino,
							start: json::unhex(start, "start")?.into(),
							end: end
								.as_deref()
								.map(|end| json::unhex(end, "end"))
								.transpose()?
								.map(Into::into),
						})
					}
				})
			})
			.collect::<Result<Vec<_>, Refusal>>()?;
		let txn = match &self.txn {
			Some(Txn::FsTxnAbort { id }) => Some(FsTxn::FsTxnAbort(FsTxnAbort {
				id: json::unhex(id, "txn id")?.into(),
			})),
			None => None,
		};
		Ok(FsEntry { ops, txn })
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Preimage {
	key: json::RowKey,
	#[serde(default)]
	value_digest: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppendBody {
	claim: Claim,
	expected_appended: Position,
	entry: Entry,
	#[serde(default)]
	preimages: Vec<Preimage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReloadBody {
	worker: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ForgetBody {
	worker: String,
	replica_incarnation: String,
	expected_appended: Position,
	#[serde(default)]
	expected_claim: Option<Claim>,
}

/// The body of a call that takes no arguments beyond its envelope.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NoBody {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RestoreBody {
	volume_key: VolumeKey,
	/// The checkpoint's name, hex.
	checkpoint: String,
	#[serde(default = "dry_run_default")]
	dry_run: bool,
	reason: String,
	/// The restore spec the dry run returned, hex; a real call resends exactly it.
	#[serde(default)]
	spec: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuditListBody {
	volume_key: VolumeKey,
	#[serde(default)]
	after: Option<String>,
	#[serde(default)]
	limit: Option<u32>,
}

impl<B> Envelope<B> {
	fn wire(&self) -> Result<RepairEnvelope, Refusal> {
		if self.reason.trim().is_empty() {
			return Err(Refusal::invalid("reason is required"));
		}
		if self.reason.len() > 4096 {
			return Err(Refusal::invalid("reason is longer than 4096 bytes"));
		}
		let plan_digest = match &self.plan_digest {
			Some(digest) => Some(
				<[u8; 32]>::try_from(json::unhex(digest, "planDigest")?)
					.map_err(|_| Refusal::invalid("planDigest is not 32 bytes"))?,
			),
			None => None,
		};
		Ok(RepairEnvelope {
			operation_id: json::uuid(&self.operation_id, "operationId")?,
			dry_run: self.dry_run,
			reason: self.reason.clone().into(),
			principal: ADMIN_PRINCIPAL.into(),
			tablet_id: json::uuid(&self.fence.tablet.tablet_id, "tabletId")?,
			tablet_grant_epoch: self.fence.tablet.tablet_grant_epoch,
			volume_log_key: self.fence.volume_log_key.wire()?,
			plan_digest,
		})
	}
}

pub(crate) async fn quarantine_rebuild(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<RebuildBody>>,
) -> RepairResult {
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairRebuildRecord(RepairRebuildRecord {
			expected_cold: body.body.expected_cold.wire(),
			allow_missing_evidence: body.body.allow_missing_evidence,
		}),
	};
	call(&ctx, request).await
}

pub(crate) async fn claim_acquire(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<AcquireBody>>,
) -> RepairResult {
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairClaimAcquire(RepairClaimAcquire {
			ttl_ms: body.body.ttl_ms,
			fence_holder: body.body.fence_holder,
			expected_appended: body.body.expected_appended.as_ref().map(Position::wire),
		}),
	};
	call(&ctx, request).await
}

pub(crate) async fn claim_release(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<ReleaseBody>>,
) -> RepairResult {
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairClaimRelease(RepairClaimRelease {
			claim: body.body.claim.wire()?,
		}),
	};
	call(&ctx, request).await
}

pub(crate) async fn append(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<AppendBody>>,
) -> RepairResult {
	let preimages = body
		.body
		.preimages
		.iter()
		.map(|preimage| {
			Ok(RepairPreimage {
				key: preimage.key.wire()?,
				value_digest: match &preimage.value_digest {
					Some(digest) => Some(
						<[u8; 32]>::try_from(json::unhex(digest, "valueDigest")?)
							.map_err(|_| Refusal::invalid("valueDigest is not 32 bytes"))?,
					),
					None => None,
				},
			})
		})
		.collect::<Result<Vec<_>, Refusal>>()?;
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairAppend(RepairAppend {
			claim: body.body.claim.wire()?,
			expected_appended: body.body.expected_appended.wire(),
			entry: body.body.entry.wire()?,
			preimages,
		}),
	};
	call(&ctx, request).await
}

pub(crate) async fn replica_reload(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<ReloadBody>>,
) -> RepairResult {
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairReplicaReload(RepairReplicaReload {
			worker: json::uuid(&body.body.worker, "worker")?,
		}),
	};
	call(&ctx, request).await
}

pub(crate) async fn replica_forget(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<ForgetBody>>,
) -> RepairResult {
	let expected_claim = body
		.body
		.expected_claim
		.as_ref()
		.map(Claim::wire)
		.transpose()?;
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairReplicaForget(RepairReplicaForget {
			worker: json::uuid(&body.body.worker, "worker")?,
			replica_incarnation: json::uuid(&body.body.replica_incarnation, "replicaIncarnation")?,
			expected_appended: body.body.expected_appended.wire(),
			expected_claim,
		}),
	};
	call(&ctx, request).await
}

pub(crate) async fn abort_unload(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<NoBody>>,
) -> RepairResult {
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairAbortUnload,
	};
	call(&ctx, request).await
}

pub(crate) async fn redrive(
	State(ctx): State<EngineContext>,
	Json(body): Json<Envelope<NoBody>>,
) -> RepairResult {
	let request = RepairRequest {
		envelope: body.wire()?,
		body: RepairBody::RepairRedrive,
	};
	call(&ctx, request).await
}

/// Restores a volume to one of its named checkpoints through the existing restore fork, whose
/// drain fences the writer and which publishes a new incarnation without a cold write. A dry
/// run resolves the checkpoint and returns the restore's spec; the real call resends that exact
/// spec, so a retry joins the same restore instead of starting a second one. The restore's own
/// maintainer log ops make it durable; it has no repair audit row of its own.
pub(crate) async fn restore(
	State(ctx): State<EngineContext>,
	Json(body): Json<RestoreBody>,
) -> RepairResult {
	use vbare::{Encode, Message};
	if body.reason.trim().is_empty() {
		return Err(Refusal::invalid("reason is required"));
	}
	let key = body.volume_key.wire()?;
	let name = json::unhex(&body.checkpoint, "checkpoint")?;
	let client = ctx.depot();
	let fork_refusal = |error: depot_client::forks::Error| match error {
		depot_client::forks::Error::Rejected(reason) => Refusal {
			status: StatusCode::CONFLICT,
			code: "FORK_REJECTED",
			message: format!("{reason:?}"),
			retry_after_ms: None,
			current: None,
		},
		depot_client::forks::Error::Refused {
			reason,
			retry_after_ms,
			..
		} => Refusal::unavailable(format!("{reason:?}"), retry_after_ms),
		depot_client::forks::Error::Unknown { spec } => Refusal {
			status: StatusCode::ACCEPTED,
			code: "UNKNOWN_OUTCOME",
			message: format!(
				"the restore may have run; resend spec {}",
				hex::encode(spec.encode_to_bytes())
			),
			retry_after_ms: Some(1000),
			current: Some(json::wire(&*spec)),
		},
		depot_client::forks::Error::Preparation(error) => {
			Refusal::unavailable(format!("{error:#}"), 100)
		}
	};
	if body.dry_run {
		let spec = client
			.prepare_checkpoint_restore(key, name)
			.await
			.map_err(fork_refusal)?;
		return Ok(Json(json!({
			"result": "PLANNED",
			"plan": json::wire(&spec),
			"spec": hex::encode(spec.encode_to_bytes()),
		})));
	}
	let Some(spec) = &body.spec else {
		return Err(Refusal::invalid(
			"a real restore resends the spec its dry run returned",
		));
	};
	let spec = depot_fs_core::FsForkSpec::decode(json::unhex(spec, "spec")?.into())
		.map_err(|_| Refusal::invalid("spec does not decode"))?;
	if spec.destination_volume_key != key || spec.parent_volume_key != key || spec.restore.is_none()
	{
		return Err(Refusal::invalid("spec is not a restore of this volume"));
	}
	tracing::warn!(
		reason = %body.reason,
		operation_id = %uuid::Uuid::from_bytes(spec.operation_id),
		"an operator is restoring a volume to a checkpoint"
	);
	let record = client.resume_fork(spec).await.map_err(fork_refusal)?;
	Ok(Json(json!({
		"result": "APPLIED",
		"record": json::wire(&record),
	})))
}

pub(crate) async fn audit_list(
	State(ctx): State<EngineContext>,
	Json(body): Json<AuditListBody>,
) -> RepairResult {
	let response = maintainer(
		&ctx,
		ClientRequest::RepairAuditListRequest(RepairAuditListRequest {
			volume_key: body.volume_key.wire()?,
			after: body
				.after
				.as_deref()
				.map(|after| json::uuid(after, "after"))
				.transpose()?,
			limit: body.limit.unwrap_or(100),
		}),
	)
	.await?;
	match response {
		ClientResponse::RepairAuditPage(page) => Ok(Json(json::wire(&page))),
		other => Err(Refusal::unavailable(
			format!("the maintainer answered with {other:?}"),
			100,
		)),
	}
}

/// Sends one mutating call and renders its response. A real call whose reply is lost may have
/// run, so it answers `UNKNOWN_OUTCOME` rather than a retryable refusal.
pub(crate) async fn call(ctx: &EngineContext, request: RepairRequest) -> RepairResult {
	let operation_id = request.envelope.operation_id;
	let dry_run = request.envelope.dry_run;
	match maintainer(ctx, ClientRequest::RepairRequest(request)).await {
		Ok(ClientResponse::RepairResponse(response)) => Ok(Json(render(&response))),
		Ok(other) => Err(Refusal::unavailable(
			format!("the maintainer answered with {other:?}"),
			100,
		)),
		// A refusal at admission ran nothing, whatever the call.
		Err(refusal) if refusal.code != "UNAVAILABLE" || dry_run => Err(refusal),
		Err(refusal) => Err(Refusal {
			status: StatusCode::ACCEPTED,
			code: "UNKNOWN_OUTCOME",
			message: format!(
				"the call may have run ({}); resend operationId {} to learn its outcome",
				refusal.message,
				uuid::Uuid::from_bytes(operation_id)
			),
			retry_after_ms: Some(1000),
			current: None,
		}),
	}
}

/// A repair response in the routes' common shape.
pub(crate) fn render(response: &RepairResponse) -> Value {
	use rivet_data::generated::depot_client_protocol_v1::RepairResult as Result;
	let operation_id = uuid::Uuid::from_bytes(response.operation_id).to_string();
	match &response.result {
		Result::RepairPlanned(planned) => json!({
			"operationId": operation_id,
			"result": "PLANNED",
			"plan": json::wire(&planned.plan),
			"planDigest": hex::encode(planned.plan_digest),
		}),
		Result::RepairApplied(applied) => json!({
			"operationId": operation_id,
			"result": if response.already_applied { "ALREADY_APPLIED" } else { "APPLIED" },
			"plan": json::wire(&applied.plan),
			"planDigest": hex::encode(applied.plan_digest),
			"detail": applied.detail.as_ref().map(json::wire),
		}),
		Result::RepairRefused(refused) => json!({
			"operationId": operation_id,
			"result": "REFUSED",
			"alreadyApplied": response.already_applied,
			"refusal": {
				"code": json::wire(&refused.code),
				"message": refused.message.to_string(),
				"retryAfterMs": refused.retry_after_ms,
				"current": refused.current.as_ref().map(json::wire),
			},
		}),
		Result::RepairUnknown(unknown) => json!({
			"operationId": operation_id,
			"result": "UNKNOWN_OUTCOME",
			"alreadyApplied": response.already_applied,
			"message": unknown.message.to_string(),
		}),
	}
}
