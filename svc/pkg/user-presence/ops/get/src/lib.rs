use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

const DEFAULT_STATUS: i32 = backend::user::Status::Offline as i32;

#[operation(name = "user-presence-get")]
async fn handle(
	ctx: OperationContext<user_presence::get::Request>,
) -> GlobalResult<user_presence::get::Response> {
	let mut redis = ctx.redis_user_presence().await?;

	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// TODO: Merge Redis calls in to single pipe

	// Fetch game activities
	let mut game_activities = {
		use util_user_presence::key;

		let mut pipe = redis::pipe();

		for user_id in &user_ids {
			pipe.hget(
				key::game_activity(*user_id),
				&[
					key::game_activity::USER_ID,
					key::game_activity::GAME_ID,
					key::game_activity::MESSAGE,
					key::game_activity::PUBLIC_METADATA_JSON,
					key::game_activity::FRIEND_METADATA_JSON,
				],
			);
		}

		pipe.query_async::<_, Vec<
			Vec<(
				Option<String>,
				Option<String>,
				Option<String>,
				Option<String>,
				Option<String>,
			)>,
		>>(&mut redis)
			.await?
			.into_iter()
			.flatten()
			.map(
				|(user_id, game_id, message, public_metadata, friend_metadata)| {
					if user_id.is_none() {
						return GlobalResult::Ok(None);
					}

					let user_id = util::uuid::parse(internal_unwrap!(user_id))?;
					let game_id = util::uuid::parse(internal_unwrap!(game_id))?;
					GlobalResult::Ok(Some((
						user_id,
						backend::user::presence::GameActivity {
							game_id: Some(game_id.into()),
							message: internal_unwrap_owned!(message),
							public_metadata,
							friend_metadata,
						},
					)))
				},
			)
			.filter_map(|x| x.transpose())
			.collect::<GlobalResult<HashMap<_, _>>>()?
	};

	// Fetch presences
	let mut user_presences = {
		use util_user_presence::key;

		let mut pipe = redis::pipe();

		for user_id in &user_ids {
			pipe.hget(
				key::user_presence(*user_id),
				&[
					key::user_presence::USER_ID,
					key::user_presence::UPDATE_TS,
					key::user_presence::STATUS,
				],
			);
		}

		pipe.query_async::<_, Vec<Vec<(Option<String>, Option<i64>, Option<i64>)>>>(&mut redis)
			.await?
			.into_iter()
			.flatten()
			.map(|(user_id, update_ts, status)| {
				if user_id.is_none() {
					return GlobalResult::Ok(None);
				}

				let user_id = util::uuid::parse(internal_unwrap!(user_id))?;
				GlobalResult::Ok(Some((
					user_id,
					backend::user::Presence {
						update_ts: internal_unwrap_owned!(update_ts),
						status: internal_unwrap_owned!(status) as i32,
						game_activity: game_activities.remove(&user_id),
					},
				)))
			})
			.filter_map(|x| x.transpose())
			.collect::<GlobalResult<HashMap<_, _>>>()?
	};

	let users = user_ids
		.into_iter()
		.map(|user_id| {
			let presence = user_presences
				.remove(&user_id)
				.unwrap_or(backend::user::Presence {
					update_ts: 0,
					status: DEFAULT_STATUS,
					game_activity: None,
				});

			Ok(user_presence::get::UserPresenceEntry {
				user_id: Some(user_id.into()),
				presence: Some(presence),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(user_presence::get::Response { users })
}
