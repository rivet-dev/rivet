use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "user-notification-auth-unregister")]
async fn handle(
	ctx: OperationContext<user_notification_auth::unregister::Request>,
) -> GlobalResult<user_notification_auth::unregister::Response> {
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	match unwrap!(user_notification_auth::unregister::request::Service::from_i32(ctx.service)) {
		user_notification_auth::unregister::request::Service::Firebase => {
			sqlx::query("DELETE FROM db_user_notification_auth.users WHERE user_id = $1")
				.bind(user_id)
				.execute(&ctx.crdb().await?)
				.await?;
		}
	}

	Ok(user_notification_auth::unregister::Response {})
}
