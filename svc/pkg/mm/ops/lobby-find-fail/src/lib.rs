use futures_util::FutureExt;
use proto::backend::pkg::*;
use redis::AsyncCommands;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "mm-lobby-find-fail")]
async fn handle(
	ctx: OperationContext<mm::lobby_find_fail::Request>,
) -> GlobalResult<mm::lobby_find_fail::Response> {
	let crdb = ctx.crdb().await?;
	let redis = ctx.redis_mm().await?;

	// Complete all queries in parallel
	let mut futs = Vec::new();
	for query_id in &ctx.query_ids {
		let query_id = query_id.as_uuid();

		let ctx = ctx.clone();
		let redis = redis.clone();

		if let Some(force_fail) = &ctx.force_fail {
			let namespace_id = unwrap_ref!(force_fail.namespace_id).as_uuid();

			tracing::info!("forcing fail event");
			futs.push(
				publish_fail_event(ctx.clone(), namespace_id, query_id, ctx.error_code).boxed(),
			);
		} else {
			tracing::info!(?query_id, "failing query");
			futs.push(fail_query(ctx.clone(), redis, query_id, ctx.error_code).boxed());
		}
	}
	futures_util::future::try_join_all(futs).await?;

	Ok(mm::lobby_find_fail::Response {})
}

// TODO: Break this down in to batch statements for each phase
#[tracing::instrument(skip(ctx, redis))]
async fn fail_query(
	ctx: OperationContext<mm::lobby_find_fail::Request>,
	mut redis: RedisPool,
	query_id: Uuid,
	error_code: i32,
) -> GlobalResult<()> {
	// Remove from Redis
	redis
		.unlink(&[
			util_mm::key::find_query_state(query_id),
			util_mm::key::find_query_player_ids(query_id),
		])
		.await?;

	// Update query status in database if pending
	let query_row = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		UPDATE db_mm_state.find_queries
		SET
			status = $3,
			error_code = $4
		WHERE query_id = $1 AND status = $2
		RETURNING namespace_id
		",
		query_id,
		util_mm::FindQueryStatus::Pending as i64,
		util_mm::FindQueryStatus::Fail as i64,
		error_code as i64,
	)
	.await?;

	// Update query status if the status hasn't already been completed
	if let Some((namespace_id,)) = query_row {
		// Publish the fail event
		publish_fail_event(ctx.clone(), namespace_id, query_id, error_code).await?;
	} else {
		tracing::info!("find query was not updated");
	}

	Ok(())
}

async fn publish_fail_event(
	ctx: OperationContext<mm::lobby_find_fail::Request>,
	namespace_id: Uuid,
	query_id: Uuid,
	error_code: i32,
) -> GlobalResult<()> {
	// Publish fail message
	msg!([ctx] mm::msg::lobby_find_fail(namespace_id, query_id) {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		error_code: error_code,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "mm.query.fail".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"namespace_id": namespace_id,
					"query_id": query_id,
					"error_code": error_code,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
