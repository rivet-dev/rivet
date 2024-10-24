use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-player-count-for-namespace")]
async fn handle(
	ctx: OperationContext<mm::player_count_for_namespace::Request>,
) -> GlobalResult<mm::player_count_for_namespace::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let mut pipe = redis::pipe();
	for namespace_id in &namespace_ids {
		pipe.zcard(util_mm::key::ns_player_ids(*namespace_id));
	}
	let player_counts = pipe
		.query_async::<_, Vec<u32>>(&mut ctx.redis_mm().await?)
		.await?;

	let namespaces = namespace_ids
		.iter()
		.zip(player_counts.iter())
		.map(
			|(ns_id, player_count)| mm::player_count_for_namespace::response::Namespace {
				namespace_id: Some((*ns_id).into()),
				player_count: *player_count,
			},
		)
		.collect::<Vec<_>>();

	Ok(mm::player_count_for_namespace::Response { namespaces })
}
