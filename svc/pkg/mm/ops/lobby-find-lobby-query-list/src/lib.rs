use proto::backend::pkg::*;
use redis::AsyncCommands;
use rivet_operation::prelude::*;

const MAX_COUNT: isize = 16;

#[operation(name = "mm-lobby-find-lobby-query-list")]
async fn handle(
	ctx: OperationContext<mm::lobby_find_lobby_query_list::Request>,
) -> GlobalResult<mm::lobby_find_lobby_query_list::Response> {
	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();

	let query_ids = ctx
		.redis_mm()
		.await?
		.zrange::<_, Vec<String>>(util_mm::key::lobby_find_queries(lobby_id), 0, MAX_COUNT - 1)
		.await?
		.iter()
		.map(String::as_str)
		.map(util::uuid::parse)
		.filter_map(Result::ok)
		.map(common::Uuid::from)
		.collect::<Vec<common::Uuid>>();

	if query_ids.len() as isize == MAX_COUNT {
		tracing::warn!(
			"too many find queries, short circuiting to prevent bad things from happening"
		);
		return Ok(mm::lobby_find_lobby_query_list::Response {
			query_ids: Vec::new(),
		});
	}

	Ok(mm::lobby_find_lobby_query_list::Response { query_ids })
}
