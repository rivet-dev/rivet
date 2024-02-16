use chirp_worker::prelude::*;
use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "team-profile-set")]
async fn worker(ctx: &OperationContext<team::msg::profile_set::Message>) -> GlobalResult<()> {
	let team::msg::profile_set::Message {
		team_id,
		display_name,
		bio,
		publicity,
	} = ctx.body();
	let raw_team_id = unwrap_ref!(team_id);
	let team_id: Uuid = raw_team_id.as_uuid();

	let mut query_components = Vec::new();

	// Check if each component exists
	if display_name.is_some() {
		query_components.push(format!("display_name = ${},", query_components.len() + 2));
	}
	if bio.is_some() {
		query_components.push(format!("bio = ${},", query_components.len() + 2));
	}
	if publicity.is_some() {
		query_components.push(format!("publicity = ${},", query_components.len() + 2));
	}

	if query_components.is_empty() {
		tracing::info!("query components are empty. nothing to do");
		return Ok(());
	}

	// Validate profile
	let validation_res = op!([ctx] team_profile_validate {
		display_name: display_name.clone(),
		bio: bio.clone()
	})
	.await?;
	if !validation_res.errors.is_empty() {
		tracing::warn!(errors = ?validation_res.errors, "validation errors");

		msg!([ctx] team::msg::profile_set_fail(team_id) {
			team_id: Some(team_id.into()),
			error_code: team::msg::profile_set_fail::ErrorCode::ValidationFailed as i32,
		})
		.await?;

		return Ok(());
	}

	// Build query
	let built_query = query_components.into_iter().collect::<String>();
	let query_string = format!(
		"UPDATE db_team.teams SET {} WHERE team_id = $1",
		built_query.trim_end_matches(',')
	);

	// TODO: Migrate to sql_execute! macro
	let query = sqlx::query(&query_string).bind(team_id);

	// Bind display name
	let query = if let Some(display_name) = display_name {
		query.bind(display_name)
	} else {
		query
	};

	// Bind bio
	let query = if let Some(bio) = bio {
		query.bind(util::format::biography(bio))
	} else {
		query
	};

	// Bind publicity
	let query = if let Some(publicity) = publicity {
		query.bind(*publicity)
	} else {
		query
	};

	query.execute(&ctx.crdb().await?).await?;

	ctx.cache().purge("team", [team_id]).await?;

	// Accept all group join requests when publicity is set to open
	if let Some(publicity) = publicity {
		let publicity = unwrap!(backend::team::Publicity::from_i32(*publicity));

		if publicity == backend::team::Publicity::Open {
			let join_request_res = op!([ctx] team_join_request_list {
				team_ids: vec![*raw_team_id],
			})
			.await?;
			let join_requests = unwrap!(join_request_res.teams.first())
				.join_requests
				.clone();

			futures_util::stream::iter(join_requests.into_iter().map(|join_request| {
				let ctx = ctx.chirp();
				let raw_team_id = *raw_team_id;

				async move {
					let user_id = unwrap_ref!(join_request.user_id).as_uuid();

					msg!([ctx] team::msg::join_request_resolve(team_id, user_id) -> team::msg::join_request_resolve_complete {
						team_id: Some(raw_team_id),
						user_id: join_request.user_id,
						resolution: true,
					})
					.await
					.map_err(Into::<GlobalError>::into)
				}
			}))
			.buffer_unordered(32)
			.try_collect::<Vec<_>>()
			.await?;
		}
	}

	tokio::try_join!(
		msg!([ctx] team::msg::update(team_id) {
			team_id: Some(team_id.into()),
		}),
		msg!([ctx] team::msg::profile_set_complete(team_id) { }),
	)?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "team.profile_set".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"display_name": display_name,
					"has_bio": bio.is_some(),
					"publicity": publicity,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
