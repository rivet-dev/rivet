//! Bounded lifecycle metadata reads, routed through the current tablet maintainer.
use super::{
	Refusal, RepairResult,
	inspect::maintainer,
	json::{self, VolumeLogKey},
};
use axum::{Json, extract::State};
use rivet_data::generated::depot_client_protocol_v1::{
	ClientRequest, ClientResponse, RepairDiagnosticsKind, RepairDiagnosticsRequest,
};
use rivet_engine_context::EngineContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Body {
	volume_log_key: VolumeLogKey,
	#[serde(default)]
	after: Option<String>,
	#[serde(default)]
	limit: Option<u32>,
}

async fn read(ctx: EngineContext, body: Body, kind: RepairDiagnosticsKind) -> RepairResult {
	let request = RepairDiagnosticsRequest {
		volume_log_key: body.volume_log_key.wire()?,
		kind,
		after: body
			.after
			.as_deref()
			.map(|v| json::unhex(v, "after").map(Into::into))
			.transpose()?,
		limit: body.limit.unwrap_or(100).clamp(1, 100),
	};
	let response = maintainer(&ctx, ClientRequest::RepairDiagnosticsRequest(request)).await?;
	match response {
		ClientResponse::RepairDiagnosticsPage(page) => Ok(Json(json::wire(&page))),
		other => Err(Refusal::unavailable(
			format!("unexpected diagnostic response: {other:?}"),
			100,
		)),
	}
}

pub(crate) async fn forks(
	State(ctx): State<EngineContext>,
	Json(body): Json<Body>,
) -> RepairResult {
	read(ctx, body, RepairDiagnosticsKind::Forks).await
}
pub(crate) async fn pins(State(ctx): State<EngineContext>, Json(body): Json<Body>) -> RepairResult {
	read(ctx, body, RepairDiagnosticsKind::Pins).await
}
pub(crate) async fn pending(
	State(ctx): State<EngineContext>,
	Json(body): Json<Body>,
) -> RepairResult {
	read(ctx, body, RepairDiagnosticsKind::Pending).await
}
pub(crate) async fn alarms(
	State(ctx): State<EngineContext>,
	Json(body): Json<Body>,
) -> RepairResult {
	read(ctx, body, RepairDiagnosticsKind::Alarms).await
}

pub(crate) async fn incarnations(
	State(ctx): State<EngineContext>,
	Json(body): Json<Body>,
) -> RepairResult {
	read(ctx, body, RepairDiagnosticsKind::Incarnations).await
}

pub(crate) async fn activity(
	State(ctx): State<EngineContext>,
	Json(body): Json<Body>,
) -> RepairResult {
	if let Some(after) = &body.after {
		let cursor = json::unhex(after, "after")?;
		let key = depot_types::VolumeLogKey(body.volume_log_key.wire()?);
		if cursor.len() != 49 || cursor[..16] != key.digest() || cursor[40] > 1 {
			return Err(Refusal::invalid("invalid or foreign activity cursor"));
		}
	}
	read(ctx, body, RepairDiagnosticsKind::Activity).await
}
