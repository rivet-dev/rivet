use proto::backend::pkg::*;
use redis::AsyncCommands;
use rivet_operation::prelude::*;
use serde_json::json;
use util_mm::FindQueryStatus;

struct FindQuery {
	namespace_id: Uuid,
	lobby_id: Uuid,
	player_ids: Vec<Uuid>,
}

/// Idempotent operation to complete the given queries if all required conditions have completed.
///
/// Will be called multiple times to check if the find is complete.
#[operation(name = "mm-lobby-find-try-complete")]
async fn handle(
	ctx: OperationContext<mm::lobby_find_try_complete::Request>,
) -> GlobalResult<mm::lobby_find_try_complete::Response> {
	let crdb = ctx.crdb().await?;
	let redis = ctx.redis_mm().await?;

	util::inject_latency!();

	// Complete all queries in parallel
	let mut futs = Vec::new();
	for query_id in &ctx.query_ids {
		let query_id = query_id.as_uuid();

		let ctx = ctx.clone();
		let redis = redis.clone();
		futs.push(complete_query(ctx, redis, query_id));
	}
	futures_util::future::try_join_all(futs).await?;

	Ok(mm::lobby_find_try_complete::Response {})
}

// TODO: Break this down in to batch statements for each phase
#[tracing::instrument(skip(ctx, redis))]
async fn complete_query(
	ctx: OperationContext<mm::lobby_find_try_complete::Request>,
	mut redis: RedisPool,
	query_id: Uuid,
) -> GlobalResult<()> {
	// Fetch & validate the query status
	let find_query = {
		use util_mm::key;

		let find_query_state_key = key::find_query_state(query_id);
		let (exists, namespace_id, lobby_id, status, player_ids) = redis::pipe()
			.exists(&find_query_state_key)
			.hget(&find_query_state_key, key::find_query_state::NAMESPACE_ID)
			.hget(&find_query_state_key, key::find_query_state::LOBBY_ID)
			.hget(&find_query_state_key, key::find_query_state::STATUS)
			.smembers(key::find_query_player_ids(query_id))
			.query_async::<_, (
				bool,
				Option<String>,
				Option<String>,
				Option<u8>,
				Vec<String>,
			)>(&mut redis)
			.await?;

		// Validate query exists
		if !exists {
			tracing::info!("could not find query in redis, likely deleted from redis with complete/fail in race condition");
			return Ok(());
		}

		// Validate query state
		if let (Some(namespace_id), Some(lobby_id), Some(status)) = (namespace_id, lobby_id, status)
		{
			// Check query state
			match FindQueryStatus::from_repr(status) {
				Some(FindQueryStatus::Pending) => {
					tracing::info!("validated pending state");
				}
				Some(FindQueryStatus::Complete | FindQueryStatus::Fail) => {
					tracing::warn!("query cannot be complete or fail when read from redis");
					return Ok(());
				}
				None => {
					tracing::warn!(%status, "unknown query status, continuing");
				}
			}

			FindQuery {
				namespace_id: util::uuid::parse(&namespace_id)?,
				lobby_id: util::uuid::parse(&lobby_id)?,
				player_ids: player_ids
					.iter()
					.map(String::as_str)
					.map(util::uuid::parse)
					.filter_map(Result::ok)
					.collect::<Vec<_>>(),
			}
		} else {
			bail!("failed to fetch all find query properties")
		}
	};

	// Check if lobby is ready
	let ready_ts = redis
		.hget::<_, _, Option<i64>>(
			util_mm::key::lobby_config(find_query.lobby_id),
			util_mm::key::lobby_config::READY_TS,
		)
		.await?;
	if let Some(ready_ts) = ready_ts {
		tracing::info!(?ready_ts, "lobby ready");
	} else {
		tracing::info!("lobby not ready");
		return Ok(());
	}

	// Remove from Redis
	redis::pipe()
		.atomic()
		.unlink(&[
			util_mm::key::find_query_state(query_id),
			util_mm::key::find_query_player_ids(query_id),
		])
		.ignore()
		.zrem(
			util_mm::key::lobby_find_queries(find_query.lobby_id),
			query_id.to_string(),
		)
		.ignore()
		.query_async::<_, ()>(&mut redis)
		.await?;

	// Update query status in database
	let should_update = sql_fetch_optional!(
		[ctx, (i64,)]
		"
		UPDATE db_mm_state.find_queries
		SET status = $3
		WHERE query_id = $1 AND status = $2
		RETURNING 1
		",
		query_id,
		FindQueryStatus::Pending as i64,
		FindQueryStatus::Complete as i64,
	)
	.await?
	.is_some();

	// Publish resulting messages
	if should_update {
		// Publish complete message
		msg!([ctx] mm::msg::lobby_find_complete(find_query.namespace_id, query_id) {
			namespace_id: Some(find_query.namespace_id.into()),
			query_id: Some(query_id.into()),
			lobby_id: Some(find_query.lobby_id.into()),
			player_ids: find_query.player_ids.iter().cloned().map(common::Uuid::from).collect(),
		})
		.await?;

		msg!([ctx] analytics::msg::event_create() {
			events: vec![
				analytics::msg::event_create::Event {
					event_id: Some(Uuid::new_v4().into()),
					name: "mm.query.complete".into(),
					namespace_id: Some(find_query.namespace_id.into()),
					properties_json: Some(serde_json::to_string(&json!({
						"query_id": query_id,
						"lobby_id": find_query.lobby_id,
						"player_count": find_query.player_ids.len(),
					}))?),
					..Default::default()
				}
			],
		})
		.await?;
	}

	Ok(())
}
