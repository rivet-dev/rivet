use std::collections::HashMap;

use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::pagination::Pagination;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct GetQuery {}

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = NamespacesRunnerConfigsGetResponse)]
pub struct GetResponse {
	pub runner_config: namespace::types::RunnerConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPath {
	pub namespace_id: Id,
	pub runner_name: String,
}

pub async fn get(ctx: ApiCtx, path: GetPath, _query: GetQuery) -> Result<GetResponse> {
	let runner_config = ctx
		.op(namespace::ops::runner_config::get_local::Input {
			runners: vec![(path.namespace_id, path.runner_name)],
		})
		.await?
		.into_iter()
		.next()
		.ok_or_else(|| namespace::errors::RunnerConfig::NotFound.build())?;

	Ok(GetResponse {
		runner_config: runner_config.config,
	})
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub limit: Option<usize>,
	pub cursor: Option<String>,
	pub variant: Option<namespace::keys::RunnerConfigVariant>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListPath {
	pub namespace_id: Id,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesRunnerConfigsListResponse)]
pub struct ListResponse {
	pub runner_configs: HashMap<String, namespace::types::RunnerConfig>,
	pub pagination: Pagination,
}

pub async fn list(ctx: ApiCtx, path: ListPath, query: ListQuery) -> Result<ListResponse> {
	ctx.op(namespace::ops::get_local::Input {
		namespace_ids: vec![path.namespace_id],
	})
	.await?
	.first()
	.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Parse variant from cursor if needed
	let (variant, after_name) = if let Some(cursor) = query.cursor {
		if let Some((variant, after_name)) = cursor.split_once(":") {
			if query.variant.is_some() {
				(query.variant, Some(after_name.to_string()))
			} else {
				(
					namespace::keys::RunnerConfigVariant::parse(variant),
					Some(after_name.to_string()),
				)
			}
		} else {
			(query.variant, None)
		}
	} else {
		(query.variant, None)
	};

	let runner_configs_res = ctx
		.op(namespace::ops::runner_config::list::Input {
			namespace_id: path.namespace_id,
			variant,
			after_name,
			limit: query.limit.unwrap_or(100),
		})
		.await?;

	let cursor = runner_configs_res
		.last()
		.map(|(name, config)| format!("{}:{}", config.variant(), name));

	Ok(ListResponse {
		// TODO: Implement ComposeSchema for FakeMap so we don't have to reallocate
		runner_configs: runner_configs_res.into_iter().collect(),
		pagination: Pagination { cursor },
	})
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct UpsertQuery {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertPath {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesRunnerConfigsUpsertRequest)]
pub struct UpsertRequest(namespace::types::RunnerConfig);

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = NamespacesRunnerConfigsUpsertResponse)]
pub struct UpsertResponse {}

pub async fn upsert(
	ctx: ApiCtx,
	path: UpsertPath,
	_query: UpsertQuery,
	body: UpsertRequest,
) -> Result<UpsertResponse> {
	ctx.op(namespace::ops::get_local::Input {
		namespace_ids: vec![path.namespace_id],
	})
	.await?
	.first()
	.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	ctx.op(namespace::ops::runner_config::upsert::Input {
		namespace_id: path.namespace_id,
		name: path.runner_name,
		config: body.0,
	})
	.await?;

	Ok(UpsertResponse {})
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct DeleteQuery {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeletePath {
	pub namespace_id: Id,
	pub runner_name: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesRunnerConfigsDeleteRequest)]
pub struct DeleteRequest(namespace::types::RunnerConfig);

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = NamespacesRunnerConfigsDeleteResponse)]
pub struct DeleteResponse {}

pub async fn delete(ctx: ApiCtx, path: DeletePath, _query: DeleteQuery) -> Result<DeleteResponse> {
	ctx.op(namespace::ops::get_local::Input {
		namespace_ids: vec![path.namespace_id],
	})
	.await?
	.first()
	.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	ctx.op(namespace::ops::runner_config::delete::Input {
		namespace_id: path.namespace_id,
		name: path.runner_name,
	})
	.await?;

	Ok(DeleteResponse {})
}
