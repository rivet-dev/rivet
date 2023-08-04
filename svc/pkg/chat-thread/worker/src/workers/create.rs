use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker(name = "chat-thread-create")]
async fn worker(ctx: OperationContext<chat_thread::msg::create::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-chat").await?;

	let request_id = internal_unwrap!(ctx.request_id).as_uuid();
	let kind = internal_unwrap!(internal_unwrap!(ctx.topic).kind);
	let create_ts = ctx.override_create_ts.unwrap_or(ctx.ts());

	// Insert the thread
	let preliminary_thread_id = Uuid::new_v4();
	let (thread_id, analytics_topic) = match kind {
		backend::chat::topic::Kind::Team(team) => {
			let team_id = internal_unwrap!(team.team_id).as_uuid();
			tracing::info!(?team_id, "creating team thread");

			// Insert new thread or return existing thread ID if created in race
			// condition
			let (thread_id,) = sqlx::query_as::<_, (Uuid,)>(indoc!(
				"
				WITH
					insert AS (
						INSERT INTO threads (thread_id, create_ts, team_team_id)
						VALUES ($1, $2, $3)
						ON CONFLICT (team_team_id)
						DO NOTHING
						RETURNING thread_id
					)
				SELECT thread_id FROM insert
				UNION
				SELECT thread_id FROM threads WHERE team_team_id = $3
				LIMIT 1
				"
			))
			.bind(preliminary_thread_id)
			.bind(create_ts)
			.bind(team_id)
			.fetch_one(&crdb)
			.await?;

			(thread_id, json!({ "team": { "team_id": team_id } }))
		}
		backend::chat::topic::Kind::Party(party) => {
			let party_id = internal_unwrap!(party.party_id).as_uuid();
			tracing::info!(?party_id, "creating party thread");

			// See above
			let (thread_id,) = sqlx::query_as::<_, (Uuid,)>(indoc!(
				"
				WITH
					insert AS (
						INSERT INTO threads (thread_id, create_ts, party_party_id)
						VALUES ($1, $2, $3)
						ON CONFLICT (party_party_id)
						DO NOTHING
						RETURNING thread_id
					)
				SELECT thread_id FROM insert
				UNION
				SELECT thread_id FROM threads WHERE party_party_id = $3
				LIMIT 1
				"
			))
			.bind(preliminary_thread_id)
			.bind(create_ts)
			.bind(party_id)
			.fetch_one(&crdb)
			.await?;

			(thread_id, json!({ "party": { "party_id": party_id } }))
		}
		backend::chat::topic::Kind::Direct(direct) => {
			let (user_a_id, user_b_id) = util::sort::id_pair(
				internal_unwrap!(direct.user_a_id).as_uuid(),
				internal_unwrap!(direct.user_b_id).as_uuid(),
			);
			tracing::info!(?user_a_id, ?user_b_id, "creating direct thread");

			// See above
			let (thread_id,) = sqlx::query_as::<_, (Uuid,)>(indoc!(
				"
				WITH
					insert AS (
						INSERT INTO threads (thread_id, create_ts, direct_user_a_id, direct_user_b_id)
						VALUES ($1, $2, $3, $4)
						ON CONFLICT (direct_user_a_id, direct_user_b_id)
						DO NOTHING
						RETURNING thread_id
					)
				SELECT thread_id FROM insert
				UNION
				SELECT thread_id FROM threads WHERE direct_user_a_id = $3 AND direct_user_b_id = $4
				LIMIT 1
				"
			))
			.bind(preliminary_thread_id)
			.bind(create_ts)
			.bind(user_a_id)
			.bind(user_b_id)
			.fetch_one(&crdb)
			.await?;

			(
				thread_id,
				json!({ "user": { "user_a_id": user_a_id, "user_b_id": user_b_id } }),
			)
		}
	};

	msg!([ctx] chat_thread::msg::create_complete(request_id) {
		request_id: Some(request_id.into()),
		thread_id: Some(thread_id.into()),
	})
	.await?;

	// Create the "Chat Created" message
	let chat_message_id = Uuid::new_v4();
	msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) {
		chat_message_id: Some(chat_message_id.into()),
		thread_id: Some(thread_id.into()),
		send_ts: create_ts,
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::ChatCreate(
				backend::chat::message_body::ChatCreate {},
			)),
		}),
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "chat.thread.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"thread_id": thread_id,
					"topic": analytics_topic,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}
