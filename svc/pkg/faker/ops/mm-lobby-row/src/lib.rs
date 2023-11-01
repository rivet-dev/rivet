use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "faker-mm-lobby-row")]
async fn handle(
	ctx: OperationContext<faker::mm_lobby_row::Request>,
) -> GlobalResult<faker::mm_lobby_row::Response> {
	let crdb = ctx.crdb().await?;
	let mut redis = ctx.redis_mm().await?;

	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();
	let lobby_group_id = unwrap_ref!(ctx.lobby_group_id).as_uuid();
	let region_id = unwrap_ref!(ctx.region_id).as_uuid();
	let run_id = unwrap_ref!(ctx.run_id).as_uuid();

	// let version_res = op!([ctx] mm_config_lobby_group_resolve_version {
	// 	lobby_group_ids: vec![lobby_group_id.into()],
	// })
	// .await?;
	// let version_id = unwrap_ref!(
	// 	unwrap!(version_res.versions.first()).lobby_group_id
	// );

	// let version_res = op!([ctx] mm_config_version_get {
	// 	version_ids: vec![version_id.clone()],
	// })
	// .await?;
	// let version = unwrap!(version_res.versions.first());
	// let lobby_group =
	// 	unwrap!(unwrap_ref!(version.config).lobby_groups.first());

	// We don't insert all required lobby information. This can be added to on
	// an as-needed basis.
	redis::pipe()
		.atomic()
		.zadd(
			util_mm::key::ns_lobby_ids(namespace_id),
			lobby_id.to_string(),
			util::timestamp::now(),
		)
		.query_async(&mut redis)
		.await?;
	sql_query!(
		[ctx]
		"
		INSERT INTO db_mm_state.lobbies (
			lobby_id,
			namespace_id,
			lobby_group_id,
			region_id,
			token_session_id,
			create_ts,
			stop_ts,
			run_id,
			create_ray_id,
			
			max_players_normal,
			max_players_direct,
			max_players_party,

			is_closed
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, false)
		",
		lobby_id,
		namespace_id,
		lobby_group_id,
		region_id,
		Uuid::new_v4(),
		ctx.create_ts.unwrap_or(ctx.ts()),
		ctx.stop_ts,
		run_id,
		Uuid::new_v4(),
		8,
		8,
		8,
	)
	.await?;

	Ok(faker::mm_lobby_row::Response {})
}
