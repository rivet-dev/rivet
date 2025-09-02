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

#[derive(Serialize, ToSchema)]
#[schema(as = NamespacesGetResponse)]
pub struct GetResponse {
	pub namespace: namespace::types::Namespace,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPath {
	pub namespace_id: Id,
}

pub async fn get(ctx: ApiCtx, path: GetPath, _query: GetQuery) -> Result<GetResponse> {
	let namespace = ctx
		.op(namespace::ops::get_local::Input {
			namespace_ids: vec![path.namespace_id],
		})
		.await?
		.namespaces
		.into_iter()
		.next()
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	Ok(GetResponse { namespace })
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ResolveForNameQuery {}

#[derive(Serialize, ToSchema)]
#[schema(as = NamespacesResolveForNameResponse)]
pub struct ResolveForNameResponse {
	pub namespace: namespace::types::Namespace,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveForNamePath {
	pub name: String,
}

pub async fn resolve_for_name(
	ctx: ApiCtx,
	path: ResolveForNamePath,
	_query: ResolveForNameQuery,
) -> Result<ResolveForNameResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_local::Input { name: path.name })
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	Ok(ResolveForNameResponse { namespace })
}

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
	pub limit: Option<usize>,
	pub cursor: Option<String>,
	pub name: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesListResponse)]
pub struct ListResponse {
	pub namespaces: Vec<namespace::types::Namespace>,
	pub pagination: Pagination,
}

#[utoipa::path(
    get,
	operation_id = "actors_list",
    path = "/namespaces",
    params(ListQuery),
    responses(
        (status = 200, body = ListResponse),
    ),
)]
pub async fn list(ctx: ApiCtx, _path: (), query: ListQuery) -> Result<ListResponse> {
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
	name: String,
	display_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = NamespacesCreateResponse)]
pub struct CreateResponse {
	pub namespace: namespace::types::Namespace,
}

#[utoipa::path(
    post,
	operation_id = "actors_create",
    path = "/namespaces",
	request_body(content = CreateRequest, content_type = "application/json"),
    responses(
        (status = 200, body = CreateResponse),
    ),
)]
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
		.namespaces
		.into_iter()
		.next()
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	Ok(CreateResponse { namespace })
}
