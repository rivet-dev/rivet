//! Operator repair routes for agents: `/depot/repair/v1/*`. They speak the Depot volume protocol
//! almost verbatim, as JSON with bytes in hex, so an agent sees every fact Depot already has.
//!
//! These routes are served on the API-peer listener like the rest of this crate, but unlike
//! the other routes each one requires the configured admin token through the shared edge
//! check (`rivet_guard_core::auth::check_edge`), before its handler runs. A request without it
//! is refused `auth.invalid_token` and reaches no worker.
//!
//! Reads are side-effect free: maintainer inspections, unfenced shard status and log reads,
//! raw row versions, checkpoint and audit listings. Mutating calls (`calls`) are fenced by the
//! tablet grant and the incarnation, default to a dry run, carry an operation id and are
//! audited in the tablet maintainer's log before they run. Refusals carry `retryAfterMs`
//! where a retry can help; only the caller retries.
//!
//! `forks`, `pins`, `pending`, `alarms`, `incarnations` and `activity` page one incarnation's
//! lifecycle metadata: `{volumeLogKey, after?, limit?}`, at most 100 rows a page. Pass the
//! returned `after` unchanged until it is null; a page that skipped rows can be empty and still
//! carry one. Stored rows come with their key, raw bytes and decoded value or decode error, so a
//! corrupt row stays visible. `activity` reads only the maintainer's current state, and its
//! cursor is bound to the tablet grant: start again after a grant change.
//!
//! `replica/forget` removes a leftover replica on a worker outside the tablet's stable replica
//! set. Take its `replicaIncarnation`, `expectedAppended` (its cold position, which must equal
//! its appended one) and `expectedClaim` from `replicas`; the shard refuses the forget unless
//! all of them still hold and the worker is still outside the planned set.

mod calls;
mod diagnostics;
mod inspect;
mod json;

use axum::{
	Json, Router,
	extract::{Request, State},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing::post,
};
use http::StatusCode;
use rivet_engine_context::EngineContext;
use rivet_guard_core::auth;
use serde_json::{Value, json};

pub fn router(ctx: EngineContext) -> Router {
	Router::new()
		.route(
			"/depot/repair/v1/volume/inspect",
			post(inspect::volume_inspect),
		)
		.route("/depot/repair/v1/placement", post(inspect::placement))
		.route("/depot/repair/v1/replicas", post(inspect::replicas))
		.route("/depot/repair/v1/diff", post(inspect::diff))
		.route("/depot/repair/v1/log/read", post(inspect::log_read))
		.route("/depot/repair/v1/rows/get", post(inspect::rows_get))
		.route("/depot/repair/v1/rows/scan", post(inspect::rows_scan))
		.route("/depot/repair/v1/checkpoints", post(inspect::checkpoints))
		.route("/depot/repair/v1/audit/list", post(calls::audit_list))
		.route(
			"/depot/repair/v1/incarnations",
			post(diagnostics::incarnations),
		)
		.route("/depot/repair/v1/forks", post(diagnostics::forks))
		.route("/depot/repair/v1/pins", post(diagnostics::pins))
		.route("/depot/repair/v1/pending", post(diagnostics::pending))
		.route("/depot/repair/v1/alarms", post(diagnostics::alarms))
		.route("/depot/repair/v1/activity", post(diagnostics::activity))
		.route(
			"/depot/repair/v1/quarantine/rebuild",
			post(calls::quarantine_rebuild),
		)
		.route("/depot/repair/v1/claim/acquire", post(calls::claim_acquire))
		.route("/depot/repair/v1/claim/release", post(calls::claim_release))
		.route("/depot/repair/v1/append", post(calls::append))
		.route(
			"/depot/repair/v1/replica/reload",
			post(calls::replica_reload),
		)
		.route(
			"/depot/repair/v1/replica/forget",
			post(calls::replica_forget),
		)
		.route(
			"/depot/repair/v1/lifecycle/abort-unload",
			post(calls::abort_unload),
		)
		.route("/depot/repair/v1/lifecycle/redrive", post(calls::redrive))
		.route("/depot/repair/v1/restore", post(calls::restore))
		.route_layer(middleware::from_fn_with_state(
			ctx.clone(),
			require_admin_token,
		))
		.with_state(ctx)
}

/// Refuses a request without the admin token before its handler runs.
async fn require_admin_token(
	State(ctx): State<EngineContext>,
	request: Request,
	next: Next,
) -> Response {
	if let Err(error) = auth::check_edge(ctx.config(), auth::transport_token(request.headers())) {
		return auth::http_rejection(&error)
			.map(axum::body::Body::from)
			.into_response();
	}
	next.run(request).await
}

/// A refused repair call. Nothing it names ran, unless `code` is `UNKNOWN_OUTCOME`.
#[derive(Debug)]
pub(crate) struct Refusal {
	pub status: StatusCode,
	pub code: &'static str,
	pub message: String,
	pub retry_after_ms: Option<u64>,
	pub current: Option<Value>,
}

impl Refusal {
	pub(crate) fn invalid(message: impl Into<String>) -> Self {
		Self {
			status: StatusCode::BAD_REQUEST,
			code: "INVALID",
			message: message.into(),
			retry_after_ms: None,
			current: None,
		}
	}

	pub(crate) fn unavailable(message: impl Into<String>, retry_after_ms: u64) -> Self {
		Self {
			status: StatusCode::SERVICE_UNAVAILABLE,
			code: "UNAVAILABLE",
			message: message.into(),
			retry_after_ms: Some(retry_after_ms),
			current: None,
		}
	}

	pub(crate) fn not_found(message: impl Into<String>) -> Self {
		Self {
			status: StatusCode::NOT_FOUND,
			code: "NOT_FOUND",
			message: message.into(),
			retry_after_ms: None,
			current: None,
		}
	}

	/// The refusal a maintainer's reject response maps to.
	pub(crate) fn rejected(
		reason: rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse,
	) -> Self {
		use rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse as Reject;
		let (status, code, retry_after_ms) = match &reason {
			Reject::NotMaintainer(reject) => (
				StatusCode::SERVICE_UNAVAILABLE,
				"NOT_MAINTAINER",
				Some(reject.retry_after_ms),
			),
			Reject::TabletSplitting => (
				StatusCode::SERVICE_UNAVAILABLE,
				"TABLET_SPLITTING",
				Some(100),
			),
			Reject::Overloaded(reject) => (
				StatusCode::SERVICE_UNAVAILABLE,
				"OVERLOADED",
				Some(reject.retry_after_ms),
			),
			Reject::VolumeUnderRepair(reject) => (
				StatusCode::SERVICE_UNAVAILABLE,
				"VOLUME_UNDER_REPAIR",
				Some(reject.retry_after_ms),
			),
			Reject::VolumeNotFound => (StatusCode::NOT_FOUND, "VOLUME_NOT_FOUND", None),
			Reject::VolumeExists => (StatusCode::CONFLICT, "VOLUME_EXISTS", None),
			Reject::VolumeDeleted => (StatusCode::GONE, "VOLUME_DELETED", None),
			Reject::VolumeRecordMissing(_) => (StatusCode::CONFLICT, "VOLUME_RECORD_MISSING", None),
		};
		Self {
			status,
			code,
			message: format!("{reason:?}"),
			retry_after_ms,
			current: Some(json::wire(&reason)),
		}
	}
}

impl IntoResponse for Refusal {
	fn into_response(self) -> Response {
		(
			self.status,
			Json(json!({
				"result": "REFUSED",
				"refusal": {
					"code": self.code,
					"message": self.message,
					"retryAfterMs": self.retry_after_ms,
					"current": self.current,
				},
			})),
		)
			.into_response()
	}
}

pub(crate) type RepairResult = std::result::Result<Json<Value>, Refusal>;
