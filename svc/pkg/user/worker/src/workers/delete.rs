use chirp_worker::prelude::*;
use futures_util::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;
use rand::Rng;
use serde_json::json;

const MESSAGE_BATCH_SIZE: usize = 256;
const UPLOAD_BATCH_SIZE: usize = 256;

#[worker(name = "user-delete")]
async fn worker(ctx: &OperationContext<user::msg::delete::Message>) -> GlobalResult<()> {
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	// Delete user identities
	{
		op!([ctx] user_identity_delete {
			user_ids: vec![user_id.into()],
		})
		.await?;
	}

	// Remove uploads
	{
		tracing::info!(?user_id, "removing uploads");
		let mut last_create_ts = 0;

		loop {
			let uploads_res = op!([ctx] upload_list_for_user {
				user_ids: vec![user_id.into()],
				anchor: Some(last_create_ts),
				limit: UPLOAD_BATCH_SIZE as u32,
			})
			.await?;
			let user = unwrap!(uploads_res.users.first());

			let request_id = Uuid::new_v4();
			msg!([ctx] upload::msg::delete(request_id) -> upload::msg::delete_complete {
				request_id: Some(request_id.into()),
				upload_ids: user.upload_ids.clone(),
			})
			.await?;

			// Update last timestamp
			if let Some(anchor) = user.anchor {
				last_create_ts = anchor;
			}

			if user.upload_ids.len() < UPLOAD_BATCH_SIZE {
				break;
			}
		}
	}

	// Remove from teams
	{
		tracing::info!(?user_id, "removing teams");

		let user_teams_res = op!([ctx] user_team_list {
			user_ids: vec![user_id.into()],
		})
		.await?;
		let user_teams = unwrap!(user_teams_res.users.first());

		let teams_res = op!([ctx] team_get {
			team_ids: user_teams.teams
				.iter()
				.map(|member| Ok(unwrap!(member.team_id)))
				.collect::<GlobalResult<Vec<_>>>()?
		})
		.await?;

		// Filter out teams where the user is the owner
		let non_owner_teams = teams_res
			.teams
			.clone()
			.into_iter()
			.filter(|team| team.owner_user_id != ctx.user_id);
		futures_util::stream::iter(non_owner_teams)
			.map(|team| {
				let team_id_proto = team.team_id;

				async move {
					let team_id = unwrap!(team_id_proto).as_uuid();

					msg!([ctx] team::msg::member_remove(team_id, user_id) -> team::msg::member_remove_complete {
						user_id: ctx.user_id,
						team_id: team_id_proto,
						silent: false,
					})
					.await
					.map_err(Into::<GlobalError>::into)
				}
			})
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;
	}

	// Redact user record
	{
		tracing::info!(?user_id, "removing user record");

		sql_execute!(
			[ctx]
			"
			UPDATE db_user.users
			SET
				display_name = $2,
				profile_id = NULL,
				bio = '',
				delete_complete_ts = $3
			WHERE user_id = $1
			",
			user_id,
			gen_display_name(),
			util::timestamp::now(),
		)
		.await?;

		ctx.cache().purge("user", [user_id]).await?;
	}

	msg!([ctx] user::msg::delete_complete(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	msg!([ctx] user::msg::update(user_id) {
		user_id: ctx.user_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "user.delete".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"deleted_user_id": user_id
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	tracing::info!(?user_id, "complete");

	Ok(())
}

fn gen_display_name() -> String {
	format!(
		"Deleted User {}",
		rand::thread_rng()
			.sample_iter(rand::distributions::Alphanumeric)
			.map(char::from)
			.take(10)
			.collect::<String>()
	)
}
