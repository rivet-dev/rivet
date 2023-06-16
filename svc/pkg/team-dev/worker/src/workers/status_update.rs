use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "team-dev-status-update")]
async fn worker(ctx: OperationContext<team_dev::msg::status_update::Message>) -> GlobalResult<()> {
	let team_id = if let Some(setup_complete) = ctx.setup_complete {
		let (team_id,) = sqlx::query_as::<_, (Uuid,)>(indoc!(
			"
				UPDATE dev_teams
				SET setup_complete_ts = $1
				WHERE stripe_customer_id = $2
				RETURNING team_id
				"
		))
		.bind(setup_complete.then(|| ctx.ts()))
		.bind(&ctx.stripe_customer_id)
		.fetch_one(&ctx.crdb("db-team-dev").await?)
		.await?;

		// Create stripe wallet

		Some(team_id)
	} else if let Some(payment_failed) = ctx.payment_failed {
		let (team_id,) = sqlx::query_as::<_, (Uuid,)>(indoc!(
			"
				UPDATE dev_teams
				SET payment_failed_ts = $1
				WHERE stripe_customer_id = $2
				RETURNING team_id
				"
		))
		.bind(payment_failed.then(|| ctx.ts()))
		.bind(&ctx.stripe_customer_id)
		.fetch_one(&ctx.crdb("db-team-dev").await?)
		.await?;

		Some(team_id)
	} else if let Some(spending_limit_reached) = ctx.spending_limit_reached {
		let (team_id,) = sqlx::query_as::<_, (Uuid,)>(indoc!(
			"
				UPDATE dev_teams
				SET spending_limit_reached_ts = $1
				WHERE stripe_customer_id = $2
				RETURNING team_id
				"
		))
		.bind(spending_limit_reached.then(|| ctx.ts()))
		.bind(&ctx.stripe_customer_id)
		.fetch_one(&ctx.crdb("db-team-dev").await?)
		.await?;

		Some(team_id)
	} else {
		tracing::error!("empty dev team status update");

		None
	};

	if let Some(team_id) = team_id {
		msg!([ctx] team_dev::msg::update(team_id) {
			team_id: Some(team_id.into()),
		})
		.await?;

		msg!([ctx] team_dev::msg::status_update_complete(&ctx.stripe_customer_id) {
			stripe_customer_id: ctx.stripe_customer_id.clone(),
			team_id: Some(team_id.into()),
		})
		.await?;
	}

	Ok(())
}
