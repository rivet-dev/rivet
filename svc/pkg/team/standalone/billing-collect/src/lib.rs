use std::sync::Arc;

use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Clone, Debug, sqlx::FromRow)]
struct DevTeam {
	team_id: Uuid,
	last_collection_ts: i64,
}

struct BillingCtx {
	ctx: OperationContext<()>,
	billing_teams: Vec<team::billing_aggregate::response::TeamBillingMetrics>,
	games: Vec<backend::game::Game>,
	current_collection_ts: i64,
}

#[tracing::instrument]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("team-billing-collect").await?;
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let client = shared_client.wrap_new("team-billing-collect");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"team-billing-collect".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		ts,
		ts,
		(),
		Vec::new(),
	);
	let crdb_pool = ctx.crdb().await?;

	let dev_teams = sqlx::query_as::<_, DevTeam>(indoc!(
		"
		SELECT team_id, last_collection_ts
		FROM db_team_dev.dev_teams
		"
	))
	.fetch_all(&crdb_pool)
	.await?
	.into_iter()
	.collect::<Vec<_>>();

	let now = ts;

	// Collect all billing metrics for all dev teams
	let billing_res = op!([ctx] team_billing_aggregate {
		teams: dev_teams.iter().map(|team| {
			team::billing_aggregate::request::TeamBillingRequest {
				team_id: Some(team.team_id.into()),
				query_start: team.last_collection_ts,
				query_end: now
			}
		}).collect::<Vec<_>>()
	})
	.await?;

	// Fetch game info
	let games_res = op!([ctx] game_get {
		game_ids: billing_res.teams
			.iter()
			.flat_map(|team| team.games.iter().map(|game| Ok(unwrap!(game.game_id))))
			.collect::<GlobalResult<Vec<_>>>()?,
	})
	.await?;

	let billing_ctx = Arc::new(BillingCtx {
		ctx: ctx.clone(),
		billing_teams: billing_res.teams.clone(),
		games: games_res.games.clone(),
		current_collection_ts: now,
	});

	for dev_team in dev_teams.into_iter() {
		let billing_ctx = billing_ctx.clone();
		let team_id = dev_team.team_id;

		// Create and process all metrics, iteratively
		match process_metrics(billing_ctx.clone(), dev_team).await {
			Ok(()) => {
				tracing::info!(%team_id, "finished processing metrics")
			}
			Err(err) => {
				tracing::error!(%team_id, ?err, "failed to process metrics");
			}
		}
	}

	Ok(())
}

/// Processes a single developer team's metrics.
async fn process_metrics(billing_ctx: Arc<BillingCtx>, dev_team: DevTeam) -> GlobalResult<()> {
	let team_billing = unwrap!(billing_ctx.billing_teams.iter().find(|team| {
		team.team_id
			.as_ref()
			.map_or(false, |id| id.as_uuid() == dev_team.team_id)
	}));

	for game in &team_billing.games {
		if !game.metrics.is_empty() {
			let non_zero_metrics = game
				.metrics
				.iter()
				.filter(|m| m.uptime != 0)
				.collect::<Vec<_>>();
			let has_metrics = !non_zero_metrics.is_empty();

			let game_info = unwrap!(billing_ctx.games.iter().find(|g| g.game_id == game.game_id));
			let subscription_id = unwrap!(game_info.subscription_id);

			futures_util::stream::iter(non_zero_metrics.into_iter().map(|rt_metrics| {
				async move {
					// TODO: Send send metrics to stripe
					GlobalResult::Ok(())
				}
			}))
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;

			// Update collection ts so we don't get collection overlap
			if has_metrics {
				sql_query!(
					[billing_ctx.ctx]
					"
					UPDATE db_team_dev.dev_teams
					SET last_collection_ts = $2
					WHERE team_id = $1
					",
					dev_team.team_id,
					billing_ctx.current_collection_ts,
				)
				.await?;
			}
		}
	}

	Ok(())
}
