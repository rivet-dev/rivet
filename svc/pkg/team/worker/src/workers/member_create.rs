use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

const MAX_TEAM_SIZE: i64 = 256;

#[worker(name = "team-member-create")]
async fn worker(ctx: &OperationContext<team::msg::member_create::Message>) -> GlobalResult<()> {
	// Idempotent

	let team_id: Uuid = unwrap_ref!(ctx.team_id).as_uuid();
	let user_id: Uuid = unwrap_ref!(ctx.user_id).as_uuid();

	// TODO: Race condition can allow brute force to create larger teams than
	// expected

	let (team_size,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		SELECT COUNT(*)
		FROM db_team.team_members
		WHERE team_id = $1
	",
		team_id,
	)
	.await?;
	if team_size >= MAX_TEAM_SIZE {
		return fail(ctx.chirp(), team_id, user_id, ctx.invitation.as_ref()).await;
	}

	let insert_query = sql_execute!(
		[ctx]
		"
		INSERT INTO db_team.team_members (team_id, user_id, join_ts)
		VALUES ($1, $2, $3)
		ON CONFLICT DO NOTHING
	",
		team_id,
		user_id,
		util::timestamp::now(),
	)
	.await?;
	if insert_query.rows_affected() == 0 {
		tracing::info!("member already inserted");
		msg!([ctx] team::msg::member_create_complete(team_id, user_id) {
			team_id: Some(team_id.into()),
			user_id: Some(user_id.into()),
		})
		.await?;
		return Ok(());
	}

	ctx.cache().purge("user_team_list", [user_id]).await?;

	// Dispatch events
	msg!([ctx] team::msg::member_create_complete(team_id, user_id) {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
	})
	.await?;
	msg!([ctx] user::msg::update(user_id) {
		user_id: Some(user_id.into()),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "team.member.create".into(),
				user_id: Some(user_id.into()),
				properties_json: Some(serde_json::to_string(&json!({
					"team_id": team_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	if let Some(invitation) = &ctx.invitation {
		msg!([ctx] analytics::msg::event_create() {
			events: vec![
				analytics::msg::event_create::Event {
					name: "team.invite.consume".into(),
					properties_json: Some(serde_json::to_string(&json!({
						"user_id": user_id,
						"team_id": team_id,
						"code": invitation.code,
					}))?),
					..Default::default()
				}
			],
		})
		.await?;
	}

	Ok(())
}

#[tracing::instrument]
async fn fail(
	client: &chirp_client::Client,
	team_id: Uuid,
	user_id: Uuid,
	invitation: Option<&team::msg::member_create::Invitation>,
) -> GlobalResult<()> {
	msg!([client] team::msg::member_create_fail(team_id, user_id) {
		team_id: Some(team_id.into()),
		user_id: Some(user_id.into()),
		error_code: team::msg::member_create_fail::ErrorCode::TeamFull as i32,
	})
	.await?;

	// Fail invitation if needed
	if let Some(invitation) = invitation {
		// TODO: This invite still counts towards the use count, since it's not
		// rolled back

		let invite_error_code = team_invite::msg::consume_fail::ErrorCode::TeamFull;

		tracing::warn!(%user_id, code = %invitation.code, ?invite_error_code, "consume fail");

		msg!([client] analytics::msg::event_create() {
			events: vec![
				analytics::msg::event_create::Event {
					name: "team.invite.consume_fail".into(),
					user_id: Some(user_id.into()),
					properties_json: Some(serde_json::to_string(&json!({
						"team_id": team_id,
						"code": invitation.code,
						"error": invite_error_code as i32,
					}))?),
					..Default::default()
				}
			],
		})
		.await?;

		msg!([client] team_invite::msg::consume_fail(&invitation.code, user_id) {
			user_id: Some(user_id.into()),
			code: invitation.code.to_string(),
			team_id: Some(team_id.into()),
			error_code: invite_error_code as i32,
		})
		.await?;
	}

	Ok(())
}
