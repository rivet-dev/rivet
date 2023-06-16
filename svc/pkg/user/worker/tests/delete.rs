use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

// TODO: Verify user identity is deleted
#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	tracing::info!(%user_id);

	msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
		user_id: Some(user_id.into()),
		namespace_id: None,
	})
	.await
	.unwrap();

	// Chat message
	{
		let chat_message_id = Uuid::new_v4();
		let user_b_id = Uuid::new_v4();

		let res = op!([ctx] chat_thread_get_or_create_for_topic {
			topic: Some(backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(
					backend::chat::topic::Direct {
						user_a_id: Some(user_id.into()),
						user_b_id: Some(user_b_id.into()),
					},
				)),
			}),
		})
		.await
		.unwrap();
		let thread_id = res.thread_id.unwrap().as_uuid();

		msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_message::msg::create_complete {
			chat_message_id: Some(chat_message_id.into()),
			thread_id: Some(thread_id.into()),
			send_ts: util::timestamp::now(),
			body: Some(backend::chat::MessageBody {
				kind: Some(backend::chat::message_body::Kind::Text(
					backend::chat::message_body::Text {
						sender_user_id: Some(user_id.into()),
						body: "Hello, world!".to_owned(),
					},
				)),
			}),
		})
		.await
		.unwrap();
	}

	// File upload
	{
		op!([ctx] upload_prepare {
			bucket: "bucket-build".into(),
			files: vec![
				backend::upload::PrepareFile {
					path: "upload.txt".into(),
					mime: Some("text/plain".into()),
					content_length: 123,
					..Default::default()
				},
			],
			user_id: Some(user_id.into()),
		})
		.await
		.unwrap();
	}

	// Teams
	let team_id1 = Uuid::new_v4();
	let team_id2 = Uuid::new_v4();
	{
		// Owner
		msg!([ctx] team::msg::create(team_id1) -> team::msg::create_complete {
			team_id: Some(team_id1.into()),
			display_name: util::faker::display_name(),
			owner_user_id: Some(user_id.into())
		})
		.await
		.unwrap();

		// Not owner
		msg!([ctx] team::msg::create(team_id2) -> team::msg::create_complete {
			team_id: Some(team_id2.into()),
			display_name: util::faker::display_name(),
			owner_user_id: Some(Uuid::new_v4().into())
		})
		.await
		.unwrap();
		msg!([ctx] team::msg::member_create(team_id2, user_id) -> team::msg::member_create_complete {
			team_id: Some(team_id2.into()),
			user_id: Some(user_id.into()),
			invitation: None,
		})
		.await
		.unwrap();
	}

	msg!([ctx] user::msg::delete(user_id) -> user::msg::delete_complete {
		user_id: Some(user_id.into()),
	})
	.await
	.unwrap();

	// Verify chat messages
	{
		let chat_messages_res = op!([ctx] chat_message_list_for_user {
			user_id: Some(user_id.into()),
			ts: util::timestamp::now(),
			count: 1,
			query_direction: chat_message::list_for_user::request::QueryDirection::Before as i32,
		})
		.await
		.unwrap();

		for chat_message in &chat_messages_res.messages {
			let body = chat_message.body.as_ref().unwrap();
			let kind = body.kind.as_ref().unwrap();

			assert!(
				matches!(
					kind,
					backend::chat::message_body::Kind::Deleted(
						backend::chat::message_body::Deleted { .. },
					)
				),
				"chat message not redacted"
			);
		}
	}

	// Verify uploads
	{
		let uploads_res = op!([ctx] upload_list_for_user {
			user_ids: vec![user_id.into()],
			anchor: None,
			limit: 1,
		})
		.await
		.unwrap();
		let user = uploads_res.users.first().unwrap();
		let uploads_res = op!([ctx] upload_get {
			upload_ids: user.upload_ids.clone(),
		})
		.await
		.unwrap();

		for upload in &uploads_res.uploads {
			assert!(upload.deleted_ts.is_some(), "upload not deleted");
		}
	}

	// Verify teams
	{
		let team_members_res = op!([ctx] team_member_list {
			team_ids: vec![team_id1.into(), team_id2.into()],
		})
		.await
		.unwrap();

		let team1 = team_members_res
			.teams
			.iter()
			.find(|team| team.team_id.unwrap().as_uuid() == team_id1)
			.unwrap();
		assert_eq!(1, team1.members.len());
		let team2 = team_members_res
			.teams
			.iter()
			.find(|team| team.team_id.unwrap().as_uuid() == team_id2)
			.unwrap();
		assert_eq!(1, team2.members.len());
	}

	// Verify user record
	{
		let (delete_complete_ts,): (Option<i64>,) =
			sqlx::query_as("SELECT delete_complete_ts FROM users WHERE user_id = $1")
				.bind(user_id)
				.fetch_one(&ctx.crdb("db-user").await.unwrap())
				.await
				.unwrap();
		assert!(delete_complete_ts.is_some(), "user not deleted");
	}
}
