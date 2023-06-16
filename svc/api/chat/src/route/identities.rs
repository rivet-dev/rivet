use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_chat_server::models;
use rivet_operation::prelude::*;

use crate::{auth::Auth, convert};

// MARK: GET /identities/{}/chat
pub async fn get_direct_thread(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetDirectThreadResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let (threads_res, user_res) = tokio::try_join!(
		op!([ctx] chat_thread_get_for_topic {
			topics: vec![backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(
					backend::chat::topic::Direct {
						user_a_id: Some(current_user_id.into()),
						user_b_id: Some(identity_id.into()),
					})
				)
			}]
		}),
		op!([ctx] user_get {
			user_ids: vec![identity_id.into()],
		}),
	)?;

	Ok(models::GetDirectThreadResponse {
		thread_id: threads_res
			.threads
			.first()
			.and_then(|thread| thread.thread_id.as_ref())
			.map(|id| id.as_uuid().to_string()),
		identity: user_res
			.users
			.first()
			.map(|user| convert::identity::handle_without_presence(&current_user_id, user))
			.transpose()?,
	})
}
