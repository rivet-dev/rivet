use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::{ApiBadRequest, ApiCtx};
use rivet_api_types::{namespaces::list::*, pagination::Pagination};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[tracing::instrument(skip_all)]
pub async fn list(ctx: ApiCtx, _path: (), query: ListQuery) -> Result<ListResponse> {
	let namespace_ids = [
		query.namespace_id,
		query
			.namespace_ids
			.map(|x| {
				x.split(',')
					.map(|s| {
						s.trim().parse::<rivet_util::Id>().map_err(|e| {
							ApiBadRequest {
								reason: format!("invalid id in `namespace_ids` query: {e}"),
							}
							.build()
						})
					})
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?
			.unwrap_or_default(),
	]
	.concat();

	// If name filter is provided, resolve and return only that namespace
	if let Some(name) = query.name {
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input { name })
			.await?;

		let namespaces = if let Some(namespace) = namespace {
			vec![namespace]
		} else {
			vec![]
		};

		Ok(ListResponse {
			namespaces,
			pagination: Pagination { cursor: None },
		})
	} else if !namespace_ids.is_empty() {
		let namespaces = ctx
			.op(namespace::ops::get_global::Input { namespace_ids })
			.await?;

		Ok(ListResponse {
			namespaces,
			pagination: Pagination { cursor: None },
		})
	} else {
		// Normal list operation without filter
		let namespaces_res = ctx
			.op(namespace::ops::list::Input { limit: query.limit })
			.await?;

		// For cursor-based pagination, we'll use the last namespace's create timestamp
		let cursor = namespaces_res
			.namespaces
			.last()
			.map(|ns| ns.create_ts.to_string());

		Ok(ListResponse {
			namespaces: namespaces_res.namespaces,
			pagination: Pagination { cursor },
		})
	}
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesCreateRequest)]
pub struct CreateRequest {
	pub name: String,
	pub display_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesCreateResponse)]
pub struct CreateResponse {
	pub namespace: rivet_types::namespaces::Namespace,
}

#[tracing::instrument(skip_all)]
pub async fn create(
	ctx: ApiCtx,
	_path: (),
	_query: (),
	body: CreateRequest,
) -> Result<CreateResponse> {
	let namespace_id = Id::new_v1(ctx.config().dc_label());

	ctx.workflow(namespace::workflows::namespace::Input {
		namespace_id,
		name: body.name.clone(),
		display_name: body.display_name.clone(),
	})
	.tag("namespace_id", namespace_id)
	.dispatch()
	.await?;

	let mut create_sub = ctx
		.subscribe::<namespace::workflows::namespace::CreateComplete>((
			"namespace_id",
			namespace_id,
		))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<namespace::workflows::namespace::Failed>(("namespace_id", namespace_id))
		.await?;

	tokio::select! {
		res = create_sub.next() => { res?; },
		res = fail_sub.next() => {
			let msg = res?;
			return Err(msg.into_body().error.build());
		}
	}

	let namespace = ctx
		.op(namespace::ops::get_local::Input {
			namespace_ids: vec![namespace_id],
		})
		.await?
		.into_iter()
		.next()
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	Ok(CreateResponse { namespace })
}
