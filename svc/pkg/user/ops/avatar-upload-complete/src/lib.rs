use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "user-avatar-upload-complete")]
async fn handle(
	ctx: OperationContext<user::avatar_upload_complete::Request>,
) -> GlobalResult<user::avatar_upload_complete::Response> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let upload_id = internal_unwrap!(ctx.upload_id).as_uuid();

	op!([ctx] upload_complete {
		upload_id: ctx.upload_id,
		bucket: Some("bucket-user-avatar".into()),
	})
	.await?;

	// Set avatar id
	sqlx::query(indoc!(
		"
		UPDATE db_user.users set profile_id = $2
		WHERE user_id = $1
		"
	))
	.bind(user_id)
	.bind(upload_id)
	.execute(&ctx.crdb().await?)
	.await?;

	ctx.cache().purge("user", [user_id]).await?;

	msg!([ctx] user::msg::update(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "user.avatar_set".into(),
				user_id: Some(user_id.into()),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(user::avatar_upload_complete::Response {})
}
