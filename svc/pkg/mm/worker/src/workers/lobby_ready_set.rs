use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;
use tokio::time::Duration;

/// HACK: Give the Traefik load balancer time to complete before considering the lobby ready.
///
/// Traefik updates every 500 ms and we give an extra 500 ms for grace.
///
/// See also svc/pkg/job-run/src/workers/create/mod.rs @ TRAEFIK_GRACE
const TRAEFIK_GRACE_MS: i64 = 1_000;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/lobby_ready_set.lua"));
}

#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	namespace_id: Uuid,
	region_id: Uuid,
	lobby_group_id: Uuid,
	create_ts: i64,
	ready_ts: Option<i64>,
}

#[worker(name = "mm-lobby-ready-set")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_ready::Message>) -> GlobalResult<()> {
	let mut redis_mm = ctx.redis_mm().await?;

	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();

	let lobby_row = sql_fetch_optional!(
		[ctx, LobbyRow]
		"
		WITH
			select_lobby AS (
				SELECT namespace_id, region_id, lobby_group_id, create_ts, ready_ts
				FROM db_mm_state.lobbies
				WHERE lobby_id = $1
			),
			_update AS (
				UPDATE db_mm_state.lobbies SET ready_ts = $2
				WHERE lobby_id = $1 AND ready_ts IS NULL
				RETURNING 1
			)
		SELECT * FROM select_lobby
		",
		lobby_id,
		ctx.ts(),
	)
	.await?;
	tracing::info!(?lobby_row, "lobby row");

	let Some(lobby_row) = lobby_row else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("lobby not found, may be race condition with insertion");
		}
	};

	// See TRAEFIK_GRACE_MS
	let traefik_grace_ms = TRAEFIK_GRACE_MS - (util::timestamp::now() - lobby_row.create_ts);
	if traefik_grace_ms > 0 {
		tracing::info!(traefik_grace_ms, "sleeping for traefik grace");
		tokio::time::sleep(Duration::from_millis(traefik_grace_ms as u64)).await;
	}

	msg!([ctx] mm::msg::lobby_ready_complete(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;

	// Update ready state
	if lobby_row.ready_ts.is_none() {
		let update_perf = ctx.perf().start("update-redis").await;
		REDIS_SCRIPT
			.arg(ctx.ts())
			.arg(lobby_id.to_string())
			.arg(ctx.ts() + util_mm::consts::PLAYER_READY_TIMEOUT)
			.key(util_mm::key::lobby_config(lobby_id))
			.key(util_mm::key::lobby_unready())
			.key(util_mm::key::player_unregistered())
			.key(util_mm::key::lobby_player_ids(lobby_id))
			.invoke_async(&mut redis_mm)
			.await?;
		update_perf.end();

		msg!([ctx] analytics::msg::event_create() {
			events: vec![
				analytics::msg::event_create::Event {
					event_id: Some(Uuid::new_v4().into()),
					name: "mm.lobby.ready".into(),
					properties_json: Some(serde_json::to_string(&json!({
						"namespace_id": lobby_row.namespace_id,
						"lobby_id": lobby_id,
						"create_ts": lobby_row.create_ts,
						"region_id": lobby_row.region_id,
						"lobby_group_id": lobby_row.lobby_group_id,
					}))?),
					..Default::default()
				}
			],
		})
		.await?;
	}

	Ok(())
}
