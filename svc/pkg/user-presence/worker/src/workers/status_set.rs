use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error("invalid status")]
	InvalidStatus,
}

#[worker(name = "user-presence-status-set")]
async fn worker(
	ctx: &OperationContext<user_presence::msg::status_set::Message>,
) -> GlobalResult<()> {
	let _crdb = ctx.crdb().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

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
		sql_execute!(
			[ctx]
			"
			UPSERT INTO db_user_presence.user_presences (user_id, user_set_status)
			VALUES ($1, $2)
			",
			user_id,
			ctx.user_set_status,
		)
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

	Ok(())
}
