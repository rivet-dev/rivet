use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

#[worker_test]
async fn empty(ctx: TestCtx) {
	msg!([ctx] analytics::msg::event_create() {
		events: vec![],
	})
	.await
	.unwrap();
}

#[worker_test]
async fn basic(ctx: TestCtx) {
	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "test.basic".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"foo": "bar",
					"hello": 123,
					"world": { "around": true },
				})).unwrap()),
				..Default::default()
			},
		],
	})
	.await
	.unwrap();

	// HACK: Message at the end of a test will get killed since it is spawned
	// int he background
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	// TODO: Check events written to db
}

#[worker_test]
async fn basic_with_metadata(ctx: TestCtx) {
	let user = op!([ctx] faker_user {}).await.unwrap();
	let game = op!([ctx] faker_game {}).await.unwrap();

	let user_id = user.user_id.unwrap().as_uuid();

	msg!([ctx] user_presence::msg::game_activity_set(user_id) -> user_presence::msg::update {
		user_id: user.user_id,
		game_activity: Some(backend::user::presence::GameActivity {
			game_id: game.game_id,
			message: "This is a test".into(),
			public_metadata: Some(serde_json::to_string(&json!({
				"hello": "world",
			})).unwrap()),
			friend_metadata: Some(serde_json::to_string(&json!({
				"foo": "bar",
			})).unwrap()),
		})
	})
	.await
	.unwrap();

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			// Event without user
			analytics::msg::event_create::Event {
				name: "test.no_game_or_user".into(),
				user_id: None,
				namespace_id: None,
				properties_json: Some(serde_json::to_string(&json!({
					"foo": "bar",
					"hello": 123,
					"world": { "around": true },
				})).unwrap()),
				..Default::default()
			},
			analytics::msg::event_create::Event {
				name: "test.with_game".into(),
				user_id: None,
				namespace_id: Some(Uuid::new_v4().into()),
				properties_json: Some(serde_json::to_string(&json!({
					"foo": "bar",
					"hello": 123,
					"world": { "around": true },
				})).unwrap()),
				..Default::default()
			},

			// Event that identifies user
			analytics::msg::event_create::Event {
				name: "test.identify_user".into(),
				user_id: user.user_id,
				namespace_id: None,
				..Default::default()
			},

		],
	})
	.await
	.unwrap();

	// HACK: Message at the end of a test will get killed since it is spawned
	// int he background
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	// TODO: Check events written to db
}
