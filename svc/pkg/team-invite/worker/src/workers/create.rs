use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "team-invite-create")]
async fn worker(ctx: &OperationContext<team_invite::msg::create::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;

	let team_id = internal_unwrap!(ctx.team_id).as_uuid();

	let code = rand::thread_rng()
		.sample_iter(rand::distributions::Alphanumeric)
		.map(char::from)
		.take(8)
		.collect::<String>();
	tracing::info!(%code, "generated code");

	let expire_ts = ctx.ttl.map(|ttl| Some(ctx.ts() + ttl));

	sqlx::query("INSERT INTO db_team_invite.invitations (code, team_id, create_ts, expire_ts, max_use_count) VALUES ($1, $2, $3, $4, $5)")
		.bind(&code)
		.bind(team_id)
		.bind(ctx.ts())
		.bind(expire_ts)
		.bind(ctx.max_use_count.map(|x| x as i64))
		.execute(&crdb)
		.await?;

	msg!([ctx] team_invite::msg::create_complete(team_id) {
		code: code.clone(),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.invite.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
					"code": code,
					"ttl": ctx.ttl,
					"use_count": ctx.max_use_count,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
