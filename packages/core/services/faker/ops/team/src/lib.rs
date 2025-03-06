use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "faker-team")]
async fn handle(
	ctx: OperationContext<faker::team::Request>,
) -> GlobalResult<faker::team::Response> {
	let mut member_user_ids = Vec::<common::Uuid>::new();

	for _ in 0..2usize {
		let user_create_res = op!([ctx] faker_user {}).await?;
		let user_id = unwrap_ref!(user_create_res.user_id);

		member_user_ids.push(*user_id);
	}

	let owner_user_id = unwrap!(member_user_ids.first());

	let team_id = ctx
		.team_id
		.map(|id| id.as_uuid())
		.unwrap_or_else(Uuid::new_v4);
	let team_id_proto = Some(Into::<common::Uuid>::into(team_id));
	msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
		team_id: team_id_proto,
		display_name: util::faker::display_name(),
		owner_user_id: Some(*owner_user_id)
	})
	.await?;

	for user_id in &member_user_ids {
		if user_id != owner_user_id {
			msg!([ctx] team::msg::member_create(team_id, user_id) -> team::msg::member_create_complete {
				team_id: team_id_proto,
				user_id: Some(*user_id),
				invitation: None,
			})
			.await?;
		}
	}

	Ok(faker::team::Response {
		team_id: team_id_proto,
		member_user_ids,
	})
}
