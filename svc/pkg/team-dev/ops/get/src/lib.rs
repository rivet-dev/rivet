use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(Clone, Debug, sqlx::FromRow)]
struct DevTeam {
	team_id: Uuid,
	create_ts: i64,

	setup_complete_ts: Option<i64>,
	payment_failed_ts: Option<i64>,
	spending_limit_reached_ts: Option<i64>,

	stripe_customer_id: Option<String>,
}

#[operation(name = "team-dev-get")]
async fn handle(
	ctx: OperationContext<team_dev::get::Request>,
) -> GlobalResult<team_dev::get::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	let crdb = ctx.crdb().await?;
	let teams = sql_fetch_all!(
		[ctx, DevTeam]
		"
			SELECT
				team_id,
				create_ts,
				setup_complete_ts,
				payment_failed_ts,
				spending_limit_reached_ts,
				stripe_customer_id
			FROM db_team_dev.dev_teams
			WHERE team_id = ANY($1)
			",
		&team_ids,
	)
	.await?;

	let cutoff_ts = ctx.ts() - util::billing::CUTOFF_DURATION;
	Ok(team_dev::get::Response {
		teams: teams
			.into_iter()
			.map(|team| {
				// Whether or not the team is able to run games
				let _active = team.setup_complete_ts.is_some()
					&& team
						.payment_failed_ts
						.map(|ts| ts > cutoff_ts)
						.unwrap_or(true) && team.spending_limit_reached_ts.is_none();

				Ok(backend::team::DevTeam {
					team_id: Some(team.team_id.into()),
					create_ts: team.create_ts,

					// TODO:
					status: backend::team::dev_team::DevStatus::Active as i32,
					active: true,
					// status: if team.setup_complete_ts.is_none() {
					// 	backend::team::dev_team::DevStatus::SetupIncomplete
					// } else if team.payment_failed_ts.is_some() {
					// 	backend::team::dev_team::DevStatus::PaymentFailed
					// } else if team.spending_limit_reached_ts.is_some() {
					// 	backend::team::dev_team::DevStatus::SpendingLimitReached
					// } else {
					// 	backend::team::dev_team::DevStatus::Active
					// } as i32,
					// active,
					setup_complete_ts: team.setup_complete_ts,
					payment_failed_ts: team.payment_failed_ts,
					spending_limit_reached_ts: team.spending_limit_reached_ts,

					stripe_customer_id: team.stripe_customer_id,
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
