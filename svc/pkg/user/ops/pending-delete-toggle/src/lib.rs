use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "pending-delete-toggle")]
async fn handle(
	ctx: OperationContext<user::pending_delete_toggle::Request>,
) -> GlobalResult<user::pending_delete_toggle::Response> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	// Verify the user is registered
	let identity = op!([ctx] user_identity_get {
		user_ids: vec![user_id.into()],
	})
	.await?;
	let identities = &internal_unwrap!(identity.users.first()).identities;
	assert_with!(!identities.is_empty(), IDENTITY_NOT_REGISTERED);

	sqlx::query("UPDATE db_user.users SET delete_request_ts = $2 WHERE user_id = $1")
		.bind(user_id)
		.bind(ctx.active.then(util::timestamp::now))
		.execute(&ctx.crdb().await?)
		.await?;

	msg!([ctx] user::msg::update(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	Ok(user::pending_delete_toggle::Response {})
}
