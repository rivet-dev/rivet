//! Fences and dispatches actor SQLite requests arriving on an envoy connection.
use super::{IndeterminateResult, Sessions};
use anyhow::{Context, Result};
use depot_volume::volume_client_machine::types::ErrorDisposition;
use rivet_engine_context::EngineContext;
use rivet_envoy_protocol as p;
use rivet_util_id::Id;

/// The actor a SQLite request addresses.
pub(crate) fn actor_id(message: &p::ToRivet) -> Result<&str> {
	Ok(match message {
		p::ToRivet::ToRivetSqliteGetPagesRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteCommitRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteCommitStageBeginRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteCommitFinalizeRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteExecRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteExecuteRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetSqliteExecuteBatchRequest(req) => &req.data.actor_id,
		p::ToRivet::ToRivetMetadata(_)
		| p::ToRivet::ToRivetEvents(_)
		| p::ToRivet::ToRivetAckCommands(_)
		| p::ToRivet::ToRivetStopping
		| p::ToRivet::ToRivetPong(_)
		| p::ToRivet::ToRivetKvRequest(_)
		| p::ToRivet::ToRivetTunnelMessage(_) => anyhow::bail!("not a SQLite request"),
	})
}

/// The epoch a SQLite request expects to own the actor, when it names one.
pub(crate) fn expected_epoch(message: &p::ToRivet) -> Option<u64> {
	match message {
		p::ToRivet::ToRivetSqliteGetPagesRequest(req) => req.data.expected_generation,
		p::ToRivet::ToRivetSqliteCommitRequest(req) => req.data.expected_generation,
		p::ToRivet::ToRivetSqliteCommitStageBeginRequest(req) => req.data.expected_generation,
		p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(req) => req.data.expected_generation,
		p::ToRivet::ToRivetSqliteCommitFinalizeRequest(req) => req.data.expected_generation,
		p::ToRivet::ToRivetSqliteExecRequest(req) => Some(u64::from(req.data.generation)),
		p::ToRivet::ToRivetSqliteExecuteRequest(req) => Some(u64::from(req.data.generation)),
		p::ToRivet::ToRivetSqliteExecuteBatchRequest(req) => Some(u64::from(req.data.generation)),
		p::ToRivet::ToRivetMetadata(_)
		| p::ToRivet::ToRivetEvents(_)
		| p::ToRivet::ToRivetAckCommands(_)
		| p::ToRivet::ToRivetStopping
		| p::ToRivet::ToRivetPong(_)
		| p::ToRivet::ToRivetKvRequest(_)
		| p::ToRivet::ToRivetTunnelMessage(_) => None,
	}
}

/// The epoch a request may run as: the one this connection owns, and only when the
/// request expects that same epoch.
fn admitted_epoch(owned: Option<u32>, message: &p::ToRivet) -> Option<u32> {
	let expected = expected_epoch(message);
	owned.filter(|epoch| expected.is_none_or(|expected| expected == u64::from(*epoch)))
}

/// Runs one SQLite request for the epoch this connection owns. Every failure becomes a
/// request-scoped error reply so one actor never closes the shared socket.
pub(crate) async fn request(
	ctx: &EngineContext,
	sessions: &Sessions,
	actor_id: &str,
	owned: Result<Option<super::OwnedEpoch>>,
	message: p::ToRivet,
) -> Result<p::ToEnvoy> {
	let id = Id::parse(actor_id)?;
	let owned = match owned {
		Ok(owned) => owned,
		Err(error) => {
			if let Some(retry_after_ms) = refused_before_dispatch(&error) {
				prove_inherited_tail(ctx, id, &error).await;
				return Ok(refused_failure(
					ReplyTo::of(&message)?,
					refusal_reason(&error),
					retry_after_ms,
				));
			}
			return failure(&message, "sqlite", "storage_error", error.to_string());
		}
	};
	let bound = owned;
	let owned = owned.map(|owned| owned.epoch);
	let Some(epoch) = admitted_epoch(owned, &message) else {
		// No epoch owns the actor here, so its session is retired. A request of a stale
		// epoch only retires sessions older than the one this connection owns.
		let through = match owned {
			Some(owned) => owned.saturating_sub(1),
			None => u32::MAX,
		};
		sessions.remove(id, through).await;
		return failure(
			&message,
			sqlite_types::HEAD_FENCE_MISMATCH_GROUP,
			sqlite_types::HEAD_FENCE_MISMATCH_CODE,
			"Actor execution ownership changed".into(),
		);
	};

	let bound = bound.context("admitted epoch without its binding")?;
	// The request moves into its session; an error reply needs only its kind and ID.
	let reply_to = ReplyTo::of(&message)?;
	let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "sqlite_request",
        wire_request_id = reply_to.request_id, wire_kind = ?reply_to.kind);
	let result = super::trace_scope::scope(
		span,
		sessions.execute(
			ctx.depot(),
			id,
			epoch,
			bound.create_unix_ms,
			bound.group,
			message,
		),
	)
	.await;

	match result {
		Ok(reply) => Ok(reply),
		Err(error)
			if error.is::<sqlite_embedded_filesystem::page_protocol::HeadFenceMismatch>() =>
		{
			Ok(reply_to.failure(
				sqlite_types::HEAD_FENCE_MISMATCH_GROUP,
				sqlite_types::HEAD_FENCE_MISMATCH_CODE,
				error.to_string(),
			))
		}
		Err(error) if error.is::<IndeterminateResult>() => {
			Ok(reply_to.failure("sqlite", "remote_indeterminate_result", error.to_string()))
		}
		Err(error) => {
			if let Some(retry_after_ms) = refused_before_dispatch(&error) {
				prove_inherited_tail(ctx, id, &error).await;
				return Ok(refused_failure(
					reply_to,
					refusal_reason(&error),
					retry_after_ms,
				));
			}
			tracing::warn!(%actor_id, ?error, "actor SQLite request failed");
			Ok(reply_to.failure("sqlite", "storage_error", error.to_string()))
		}
	}
}

/// Has the ancestor's tail proven when `error` refused the open of an inherited actor's
/// database because its lazy fork waits for that tail: the ancestor's actor is quiet under a
/// live holder, so nothing else proves it. Runs once per such refusal, before the refusal is
/// answered, so the runtime's next attempt finds the tail proven; every other error costs
/// nothing.
async fn prove_inherited_tail(ctx: &EngineContext, actor_id: Id, error: &anyhow::Error) {
	let Some(ancestor_database) = depot_client::tail_unproven_parent(error) else {
		return;
	};
	pegboard::actor_machine::executor::prove_inherited_database_tail(
		ctx,
		actor_id,
		ancestor_database,
	)
	.await;
}

/// The retry hint of a request depot refused before running it: a write the volume client
/// refused before dispatching it, or an open or lifecycle request the maintainer or a quorum
/// refused with a hint. Nothing ran, so the request may be sent again after the hint.
fn refused_before_dispatch(error: &anyhow::Error) -> Option<u64> {
	if let Some(retry_after_ms) = depot_client::retry_after_ms(error) {
		return Some(retry_after_ms);
	}
	let Some(depot_client::filesystem::Error::Log(error)) =
		error.downcast_ref::<depot_client::filesystem::Error>()
	else {
		return None;
	};
	if error.disposition() != ErrorDisposition::NotExecuted {
		return None;
	}
	Some(
		error
			.retry_after_ms()
			.unwrap_or(REFUSED_WRITE_RETRY_AFTER_MS),
	)
}

/// Builds the request-scoped error reply for a SQLite request.
/// The reply to a request of `epoch` after that epoch stopped on this connection. It
/// is a head fence mismatch, so the runtime treats it as lost ownership.
pub(crate) fn stopped_failure(request: &p::ToRivet, epoch: u64) -> Result<p::ToEnvoy> {
	failure(
		request,
		sqlite_types::HEAD_FENCE_MISMATCH_GROUP,
		sqlite_types::HEAD_FENCE_MISMATCH_CODE,
		format!("actor stopped: epoch {epoch} stopped on this envoy"),
	)
}

/// An inbound request whose actor has no allocation on this Envoy owns no storage session.
pub(crate) fn not_running_failure(request: &p::ToRivet) -> Result<p::ToEnvoy> {
	failure(
		request,
		sqlite_types::HEAD_FENCE_MISMATCH_GROUP,
		sqlite_types::HEAD_FENCE_MISMATCH_CODE,
		"actor is not running on this envoy".into(),
	)
}

/// The reply to a request refused because the actor's storage lane was full: a typed
/// `depot.overloaded` error naming its retry hint, so the runtime may send it again.
pub(crate) fn overloaded_failure(request: &p::ToRivet, retry_after_ms: u64) -> Result<p::ToEnvoy> {
	failure(
		request,
		"depot",
		"overloaded",
		format!(
			"actor storage lane is full (reason=actor_storage_lane, retry_after_ms={retry_after_ms})"
		),
	)
}

/// Refused before entering the shared inbound dispatcher or touching storage.
pub(crate) fn inbound_overloaded_failure(
	request: &p::ToRivet,
	retry_after_ms: u64,
) -> Result<p::ToEnvoy> {
	failure(
		request,
		"depot",
		"overloaded",
		format!(
			"node inbound queue is full (reason=node_inbound, retry_after_ms={retry_after_ms})"
		),
	)
}

/// How long the runtime waits before sending a write the volume client refused without a hint,
/// such as a missing route or an entry the shards cannot accept.
const REFUSED_WRITE_RETRY_AFTER_MS: u64 = 100;

/// The `reason` a refused request's reply names: `version_unavailable` for a write the
/// selected entry version cannot carry, `client_budget` for a request a volume client budget
/// or window refused, `write_refused` for every other refusal.
fn refusal_reason(error: &anyhow::Error) -> &'static str {
	match error.downcast_ref::<depot_client::filesystem::Error>() {
		Some(depot_client::filesystem::Error::VersionUnavailable { .. }) => "version_unavailable",
		Some(depot_client::filesystem::Error::Overloaded) => "client_budget",
		Some(_) | None => "write_refused",
	}
}

/// The reply to a request whose write depot refused before dispatching it: a typed
/// `depot.overloaded` error naming its reason and retry hint, so the runtime may send it again.
/// A write the selected entry version cannot carry answers this code too, not
/// `topology.version_unavailable`, because released RivetKit runtimes retry only
/// `depot.overloaded`; the reason in the message tells them apart.
fn refused_failure(reply_to: ReplyTo, reason: &str, retry_after_ms: u64) -> p::ToEnvoy {
	reply_to.failure(
		"depot",
		"overloaded",
		format!(
			"volume request refused before dispatch (reason={reason}, retry_after_ms={retry_after_ms})"
		),
	)
}

fn failure(request: &p::ToRivet, group: &str, code: &str, message: String) -> Result<p::ToEnvoy> {
	Ok(ReplyTo::of(request)?.failure(group, code, message))
}

/// Where a SQLite request's reply goes: its kind and request ID. It is all an error reply
/// needs, so it is taken before the request moves into its session instead of a copy of it.
#[derive(Clone, Copy)]
struct ReplyTo {
	kind: RequestKind,
	request_id: u32,
}

#[derive(Clone, Copy, Debug)]
enum RequestKind {
	GetPages,
	Commit,
	CommitStageBegin,
	CommitStageSegment,
	CommitFinalize,
	Exec,
	Execute,
	ExecuteBatch,
}

impl ReplyTo {
	fn of(request: &p::ToRivet) -> Result<Self> {
		let (kind, request_id) = match request {
			p::ToRivet::ToRivetSqliteGetPagesRequest(req) => {
				(RequestKind::GetPages, req.request_id)
			}
			p::ToRivet::ToRivetSqliteCommitRequest(req) => (RequestKind::Commit, req.request_id),
			p::ToRivet::ToRivetSqliteCommitStageBeginRequest(req) => {
				(RequestKind::CommitStageBegin, req.request_id)
			}
			p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(req) => {
				(RequestKind::CommitStageSegment, req.request_id)
			}
			p::ToRivet::ToRivetSqliteCommitFinalizeRequest(req) => {
				(RequestKind::CommitFinalize, req.request_id)
			}
			p::ToRivet::ToRivetSqliteExecRequest(req) => (RequestKind::Exec, req.request_id),
			p::ToRivet::ToRivetSqliteExecuteRequest(req) => (RequestKind::Execute, req.request_id),
			p::ToRivet::ToRivetSqliteExecuteBatchRequest(req) => {
				(RequestKind::ExecuteBatch, req.request_id)
			}
			p::ToRivet::ToRivetMetadata(_)
			| p::ToRivet::ToRivetEvents(_)
			| p::ToRivet::ToRivetAckCommands(_)
			| p::ToRivet::ToRivetStopping
			| p::ToRivet::ToRivetPong(_)
			| p::ToRivet::ToRivetKvRequest(_)
			| p::ToRivet::ToRivetTunnelMessage(_) => anyhow::bail!("not a SQLite request"),
		};
		Ok(Self { kind, request_id })
	}

	/// The request-scoped error reply.
	fn failure(self, group: &str, code: &str, message: String) -> p::ToEnvoy {
		let error = p::SqliteErrorResponse {
			group: group.to_string().into(),
			code: code.to_string().into(),
			message: message.into(),
		};
		let request_id = self.request_id;
		macro_rules! sql {
			($outer:ident, $inner:ident) => {
				p::ToEnvoy::$outer(p::$outer {
					request_id,
					data: p::$inner::SqliteErrorResponse(error),
				})
			};
		}

		match self.kind {
			RequestKind::GetPages => sql!(ToEnvoySqliteGetPagesResponse, SqliteGetPagesResponse),
			RequestKind::Commit => sql!(ToEnvoySqliteCommitResponse, SqliteCommitResponse),
			RequestKind::CommitStageBegin => sql!(
				ToEnvoySqliteCommitStageBeginResponse,
				SqliteCommitStageBeginResponse
			),
			RequestKind::CommitStageSegment => sql!(
				ToEnvoySqliteCommitStageSegmentResponse,
				SqliteCommitStageSegmentResponse
			),
			RequestKind::CommitFinalize => sql!(
				ToEnvoySqliteCommitFinalizeResponse,
				SqliteCommitFinalizeResponse
			),
			RequestKind::Exec => sql!(ToEnvoySqliteExecResponse, SqliteExecResponse),
			RequestKind::Execute => sql!(ToEnvoySqliteExecuteResponse, SqliteExecuteResponse),
			RequestKind::ExecuteBatch => sql!(
				ToEnvoySqliteExecuteBatchResponse,
				SqliteExecuteBatchResponse
			),
		}
	}
}
