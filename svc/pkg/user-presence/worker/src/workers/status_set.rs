use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("invalid status")]
	InvalidStatus,
}

#[worker(name = "user-presence-status-set")]
async fn worker(
	ctx: OperationContext<user_presence::msg::status_set::Message>,
) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-user-presence").await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	if backend::user::Status::from_i32(ctx.status).is_none() {
		return Err(Error::InvalidStatus.into());
	}

	// Set user status
	redis::cmd("HSET")
		.arg(util_user_presence::key::user_presence(user_id))
		.arg(util_user_presence::key::user_presence::USER_ID)
		.arg(user_id.to_string())
		.arg(util_user_presence::key::user_presence::UPDATE_TS)
		.arg(ctx.ts())
		.arg(util_user_presence::key::user_presence::STATUS)
		.arg(ctx.status)
		.query_async::<_, ()>(&mut ctx.redis_user_presence().await?)
		.await?;

	// Update the default status
	if ctx.user_set_status {
		sqlx::query(indoc!(
			"
			UPSERT INTO user_presences (user_id, user_set_status)
			VALUES ($1, $2)
			"
		))
		.bind(user_id)
		.bind(ctx.user_set_status)
		.execute(&crdb)
		.await?;
	}

	if !ctx.silent {
		msg!([ctx] user_presence::msg::update(user_id) {
			user_id: Some(user_id.into()),
			update_ts: ctx.ts(),
			kind: Some(user_presence::msg::update::message::Kind::Status(ctx.status)),
		})
		.await?;
	}

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "user.status_set".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": user_id,
					"status": ctx.status,
					"silent": ctx.silent,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
