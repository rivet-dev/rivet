use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-lobby-list-for-namespace")]
async fn handle(
	ctx: OperationContext<mm::lobby_list_for_namespace::Request>,
) -> GlobalResult<mm::lobby_list_for_namespace::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let mut pipe = redis::pipe();
	for namespace_id in &namespace_ids {
		pipe.zrange(util_mm::key::ns_lobby_ids(*namespace_id), 0, -1);
	}
	let ns_lobby_ids = pipe
		.query_async::<_, Vec<Vec<String>>>(&mut ctx.redis_mm().await?)
		.await?;

	let namespaces = namespace_ids
		.iter()
		.zip(ns_lobby_ids.iter())
		.map(
			|(namespace_ids, lobby_ids)| mm::lobby_list_for_namespace::response::Namespace {
				namespace_id: Some((*namespace_ids).into()),
				lobby_ids: lobby_ids
					.iter()
					.filter_map(|x| util::uuid::parse(x).ok())
					.map(Into::into)
					.collect(),
			},
		)
		.collect::<Vec<_>>();

	Ok(mm::lobby_list_for_namespace::Response { namespaces })
}
