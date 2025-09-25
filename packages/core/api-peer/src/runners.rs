use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::{pagination::Pagination, runners::list::*};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[utoipa::path(
    get,
	operation_id = "runners_list",
    path = "/runners",
    params(ListQuery),
    responses(
        (status = 200, body = ListResponse),
    ),
)]
pub async fn list(ctx: ApiCtx, _path: (), query: ListQuery) -> Result<ListResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	if let Some(runner_ids) = query.runner_ids {
		let runner_ids = runner_ids
			.split(',')
			.filter_map(|s| s.trim().parse::<rivet_util::Id>().ok())
			.collect::<Vec<_>>();
		let runners = ctx
			.op(pegboard::ops::runner::get::Input { runner_ids })
			.await?
			.runners;

		Ok(ListResponse {
			runners,
			pagination: Pagination { cursor: None },
		})
	} else {
		let list_res = ctx
			.op(pegboard::ops::runner::list_for_ns::Input {
				namespace_id: namespace.namespace_id,
				name: query.name,
				include_stopped: query.include_stopped.unwrap_or(false),
				created_before: query
					.cursor
					.as_deref()
					.map(|c| c.parse::<i64>())
					.transpose()?,
				limit: query.limit.unwrap_or(100),
			})
			.await?;

		let cursor = list_res.runners.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			runners: list_res.runners,
			pagination: Pagination { cursor },
		})
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListNamesQuery {
	pub namespace: String,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnersListNamesResponse)]
pub struct ListNamesResponse {
	pub names: Vec<String>,
	pub pagination: Pagination,
}

pub async fn list_names(
	ctx: ApiCtx,
	_path: (),
	query: ListNamesQuery,
) -> Result<ListNamesResponse> {
	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// List runner names from pegboard
	let list_res = ctx
		.op(pegboard::ops::runner::list_names::Input {
			namespace_id: namespace.namespace_id,
			after_name: query.cursor.clone(),
			limit: query.limit.unwrap_or(100),
		})
		.await?;

	let cursor = list_res.names.last().map(|x| x.to_string());

	Ok(ListNamesResponse {
		names: list_res.names,
		pagination: Pagination { cursor },
	})
}
