use anyhow::Result;
use namespace::utils::runner_config_variant;
use rivet_api_builder::ApiCtx;
use rivet_api_types::{pagination::Pagination, runner_configs::list::*};
use rivet_types::keys::namespace::runner_config::RunnerConfigVariant;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[tracing::instrument(skip_all)]
pub async fn list(ctx: ApiCtx, _path: ListPath, query: ListQuery) -> Result<ListResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let runner_names = [
		query.runner_name,
		query
			.runner_names
			.map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
			.unwrap_or_default(),
	]
	.concat();

	if !runner_names.is_empty() {
		let runner_configs = ctx
			.op(namespace::ops::runner_config::get::Input {
				runners: runner_names
					.into_iter()
					.map(|name| (namespace.namespace_id, name))
					.collect(),
				bypass_cache: false,
			})
			.await?;

		Ok(ListResponse {
			// TODO: Implement ComposeSchema for FakeMap so we don't have to reallocate
			runner_configs: runner_configs
				.into_iter()
				.map(|c| (c.name, c.config))
				.collect(),
			pagination: Pagination { cursor: None },
		})
	} else {
		// Parse variant from cursor if needed
		let (variant, after_name) = if let Some(cursor) = query.cursor {
			if let Some((variant, after_name)) = cursor.split_once(":") {
				if query.variant.is_some() {
					(query.variant, Some(after_name.to_string()))
				} else {
					(
						RunnerConfigVariant::parse(variant),
						Some(after_name.to_string()),
					)
				}
			} else {
				(query.variant, None)
			}
		} else {
			(query.variant, None)
		};

		let runner_configs = ctx
			.op(namespace::ops::runner_config::list::Input {
				namespace_id: namespace.namespace_id,
				variant,
				after_name,
				limit: query.limit.unwrap_or(100),
			})
			.await?;

		let cursor = runner_configs
			.last()
			.map(|(name, config)| format!("{}:{}", runner_config_variant(&config), name));

		Ok(ListResponse {
			// TODO: Implement ComposeSchema for FakeMap so we don't have to reallocate
			runner_configs: runner_configs.into_iter().collect(),
			pagination: Pagination { cursor },
		})
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct UpsertQuery {
	pub namespace: String,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UpsertPath {
	pub runner_name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertRequest(pub rivet_api_types::namespaces::runner_configs::RunnerConfig);

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = RunnerConfigsUpsertResponse)]
pub struct UpsertResponse {
	pub endpoint_config_changed: bool,
}

#[tracing::instrument(skip_all)]
pub async fn upsert(
	ctx: ApiCtx,
	path: UpsertPath,
	query: UpsertQuery,
	body: UpsertRequest,
) -> Result<UpsertResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let endpoint_config_changed = ctx
		.op(namespace::ops::runner_config::upsert::Input {
			namespace_id: namespace.namespace_id,
			name: path.runner_name,
			config: body.0.into(),
		})
		.await?;

	Ok(UpsertResponse {
		endpoint_config_changed,
	})
}

#[derive(Debug, Serialize, Clone, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct DeleteQuery {
	pub namespace: String,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeletePath {
	pub runner_name: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = RunnerConfigsDeleteResponse)]
pub struct DeleteResponse {}

#[tracing::instrument(skip_all)]
pub async fn delete(ctx: ApiCtx, path: DeletePath, query: DeleteQuery) -> Result<DeleteResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	ctx.op(namespace::ops::runner_config::delete::Input {
		namespace_id: namespace.namespace_id,
		name: path.runner_name,
	})
	.await?;

	Ok(DeleteResponse {})
}
