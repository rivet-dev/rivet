use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_a_id = Uuid::new_v4();
	let user_nonexistent_id = Uuid::new_v4();

	let user_a_status = backend::user::Status::Away;
	let user_a_game_id = Uuid::new_v4();

	msg!([ctx] user_presence::msg::status_set(user_a_id) -> user_presence::msg::update {
		user_id: Some(user_a_id.into()),
		status: user_a_status as i32,
		user_set_status: false,
		silent: false,
	})
	.await
	.unwrap();

	msg!([ctx] user_presence::msg::game_activity_set(user_a_id) -> user_presence::msg::update {
		user_id: Some(user_a_id.into()),
		game_activity: Some(backend::user::presence::GameActivity {
			game_id: Some(user_a_game_id.into()),
			message: "".to_owned(),
			public_metadata: None,
			friend_metadata: None
		})
	})
	.await
	.unwrap();

	let res = op!([ctx] user_presence_get {
		user_ids: vec![user_a_id.into(), user_nonexistent_id.into()],
	})
	.await
	.unwrap();
	assert_eq!(2, res.users.len());

	for user in &res.users {
		let user_id: Uuid = **user.user_id.as_ref().unwrap();
		let user_presence = user.presence.as_ref().unwrap();

		if user_id == user_a_id {
			let game_activity = user_presence.game_activity.as_ref().unwrap();

			assert_eq!(user_a_status as i32, user_presence.status);
			assert_eq!(user_a_game_id, **game_activity.game_id.as_ref().unwrap());
		} else if user_id == user_nonexistent_id {
			assert_eq!(backend::user::Status::Offline as i32, user_presence.status);
			assert!(user_presence.game_activity.is_none());
		} else {
			panic!("unknown user");
		}
	}
}
