use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[operation(name = "user-avatar-upload-complete")]
async fn handle(
	ctx: OperationContext<user::avatar_upload_complete::Request>,
) -> GlobalResult<user::avatar_upload_complete::Response> {
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();
	let upload_id = unwrap_ref!(ctx.upload_id).as_uuid();

	op!([ctx] upload_complete {
		upload_id: ctx.upload_id,
		bucket: Some("bucket-user-avatar".into()),
	})
	.await?;

	// Set avatar id
	sql_execute!(
		[ctx]
		"
		UPDATE db_user.users set profile_id = $2
		WHERE user_id = $1
		",
		user_id,
		upload_id,
	)
	.await?;

	ctx.cache().purge("user", [user_id]).await?;

	msg!([ctx] user::msg::update(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "user.avatar_set".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": user_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(user::avatar_upload_complete::Response {})
}
