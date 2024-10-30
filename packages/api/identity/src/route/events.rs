use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_convert::fetch;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /events/live
pub async fn events(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityWatchEventsResponse> {
	let current_user_id = ctx.auth().user(ctx.op_ctx()).await?.user_id;

	// Wait for an update if needed
	let EventsWaitResponse {
		removed_team_ids: _,
		user_update_ts,
		last_update_ts,
		valid_anchor: _,
	} = if let Some(anchor) = &watch_index.to_consumer()? {
		events_wait(&ctx, anchor, current_user_id).await?
	} else {
		EventsWaitResponse {
			removed_team_ids: Vec::new(),

			user_update_ts: None,
			last_update_ts: None,
			valid_anchor: false,
		}
	};
	let last_update_ts = last_update_ts.unwrap_or_else(util::timestamp::now);

	// Process events
	let events = if let Some(ts) = user_update_ts {
		vec![process_user_update_events(&ctx, ts, current_user_id).await?]
	} else {
		Vec::new()
	};

	Ok(models::IdentityWatchEventsResponse {
		events,
		watch: WatchResponse::new_as_model(last_update_ts),
	})
}

struct EventsWaitResponse {
	removed_team_ids: Vec<(i64, Uuid)>,

	/// Timestamp of the last message that was received from the tail. This is
	/// used to build the new watch index.
	last_update_ts: Option<i64>,

	/// Timestamp of the last user update.
	user_update_ts: Option<i64>,

	/// If the provided anchor was valid. If false, the tail will return
	/// immediately and we'll do a fresh pull of all events.
	valid_anchor: bool,
}

// MARK: Wait
async fn events_wait(
	ctx: &Ctx<Auth>,
	anchor: &chirp_client::TailAnchor,
	current_user_id: Uuid,
) -> GlobalResult<EventsWaitResponse> {
	// TODO: Watch for changes in direct chats, teams, and parties. i.e. profile
	// changes, activity changes, party updates, etc. This can be done by a
	// worker that publishes user-specific updates for all present users.

	let thread_tail =
		tail_all!([ctx, anchor, chirp_client::TailAllConfig::wait_return_immediately()] user::msg::event(current_user_id))
			.await?;

	let last_update_ts = thread_tail.messages.last().map(|msg| msg.msg_ts());

	tracing::info!(?thread_tail, "thread tail");

	// Decode messages
	let mut removed_team_ids = Vec::new();
	let mut user_update_ts = None;
	for msg in thread_tail.messages {
		let event = unwrap_ref!(msg.event);

		if let Some(event) = &event.kind {
			match event {
				backend::user::event::event::Kind::UserUpdate(_) => {
					user_update_ts = Some(msg.msg_ts());
				}
				backend::user::event::event::Kind::TeamMemberRemove(team) => {
					removed_team_ids.push((msg.msg_ts(), unwrap_ref!(team.team_id).as_uuid()));
				}
			}
		} else {
			tracing::warn!(?event, "unknown user event kind");
		}
	}

	Ok(EventsWaitResponse {
		removed_team_ids,
		user_update_ts,
		last_update_ts,
		valid_anchor: thread_tail.anchor_status != chirp_client::TailAllAnchorStatus::Expired,
	})
}

// MARK: User update
async fn process_user_update_events(
	ctx: &Ctx<Auth>,
	ts: i64,
	current_user_id: Uuid,
) -> GlobalResult<models::IdentityGlobalEvent> {
	let identities =
		fetch::identity::profiles(ctx.op_ctx(), current_user_id, vec![current_user_id]).await?;
	let profile = unwrap_with!(identities.into_iter().next(), IDENTITY_NOT_FOUND);

	Ok(models::IdentityGlobalEvent {
		ts: util::timestamp::to_string(ts)?,
		kind: Box::new(models::IdentityGlobalEventKind {
			identity_update: Some(Box::new(models::IdentityGlobalEventIdentityUpdate {
				identity: Box::new(profile),
			})),
			..Default::default()
		}),
		notification: None,
	})
}
