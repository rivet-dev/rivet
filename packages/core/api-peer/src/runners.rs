use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::{
	pagination::Pagination,
	runners::{get::*, list::*},
};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPath {
	pub runner_id: Id,
}

pub async fn get(ctx: ApiCtx, path: GetPath, query: GetQuery) -> Result<GetResponse> {
	let runners_res = ctx
		.op(pegboard::ops::runner::get::Input {
			runner_ids: vec![path.runner_id],
		})
		.await?;

	let runner = runners_res
		.runners
		.into_iter()
		.next()
		.ok_or_else(|| pegboard::errors::Runner::NotFound.build())?;

	// If namespace is provided, verify the runner has actors from that namespace
	if let Some(namespace_name) = query.namespace {
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: namespace_name,
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		if runner.namespace_id != namespace.namespace_id {
			return Err(pegboard::errors::Runner::NotFound.build());
		}
	}

	Ok(GetResponse { runner })
}

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
