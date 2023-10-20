use proto::backend::pkg::*;
use redis::AsyncCommands;
use rivet_operation::prelude::*;

#[operation(name = "user-presence-touch")]
async fn handle(
	ctx: OperationContext<user_presence::touch::Request>,
) -> GlobalResult<user_presence::touch::Response> {
	let mut redis = ctx.redis_user_presence().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	let elements_added: i64 = redis
		.zadd(
			util_user_presence::key::user_presence_touch(),
			user_id.to_string(),
			ctx.ts(),
		)
		.await?;
	tracing::info!(?elements_added, ?user_id, "updated presence");

	if elements_added > 0 {
		tracing::info!("publishing arrive");
		msg!([ctx] user_presence::msg::arrive(user_id) {
			user_id: Some(user_id.into()),
			silent: ctx.silent,
		})
		.await?;
	}

	Ok(user_presence::touch::Response {})
}
