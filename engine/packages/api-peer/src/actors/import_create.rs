use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_types::actors::CrashPolicy;
use rivet_util::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCreateQuery {
	pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCreateRequest {
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,
	pub create_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCreateResponse {
	pub actor: rivet_types::actors::Actor,
}

#[tracing::instrument(skip_all)]
pub async fn create(
	ctx: ApiCtx,
	_path: (),
	query: ImportCreateQuery,
	body: ImportCreateRequest,
) -> Result<ImportCreateResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace,
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let res = ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id: body.actor_id,
			namespace_id: namespace.namespace_id,
			name: body.name,
			key: body.key,
			runner_name_selector: body.runner_name_selector,
			crash_policy: body.crash_policy,
			input: None,
			forward_request: false,
			datacenter_name: None,
			start_immediately: false,
			create_ts: Some(body.create_ts),
		})
		.await?;

	Ok(ImportCreateResponse { actor: res.actor })
}
