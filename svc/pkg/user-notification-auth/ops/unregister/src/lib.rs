use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "user-notification-auth-unregister")]
async fn handle(
	ctx: OperationContext<user_notification_auth::unregister::Request>,
) -> GlobalResult<user_notification_auth::unregister::Response> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	match internal_unwrap_owned!(
		user_notification_auth::unregister::request::Service::from_i32(ctx.service)
	) {
		user_notification_auth::unregister::request::Service::Firebase => {
			sqlx::query("DELETE FROM users WHERE user_id = $1")
				.bind(user_id)
				.execute(&ctx.crdb("db-user-notification-auth").await?)
				.await?;
		}
	}

	Ok(user_notification_auth::unregister::Response {})
}
