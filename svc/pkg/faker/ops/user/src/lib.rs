use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "faker-user")]
async fn handle(
	ctx: OperationContext<faker::user::Request>,
) -> GlobalResult<faker::user::Response> {
	let user_id = Uuid::new_v4();

	msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
		user_id: Some(user_id.into()),
		namespace_id: None,
	})
	.await?;

	Ok(faker::user::Response {
		user_id: Some(user_id.into()),
	})
}
