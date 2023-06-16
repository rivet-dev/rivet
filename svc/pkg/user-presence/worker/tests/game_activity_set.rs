use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	let game_id = Uuid::new_v4();

	let mut user_presence_sub = subscribe!([ctx] user_presence::msg::update(user_id))
		.await
		.unwrap();
	msg!([ctx] user_presence::msg::game_activity_set(user_id) {
		user_id: Some(user_id.into()),
		game_activity: Some(backend::user::presence::GameActivity {
			game_id: Some(game_id.into()),
			message: "".to_owned(),
			public_metadata: None,
			friend_metadata: None
		})
	})
	.await
	.unwrap();
	user_presence_sub.next().await.unwrap();

	let mut redis = ctx.redis_user_presence().await.unwrap();
	let redis_game_id: Option<Uuid> = redis
		.hget::<_, _, Option<String>>(
			util_user_presence::key::game_activity(user_id),
			util_user_presence::key::game_activity::GAME_ID,
		)
		.await
		.unwrap()
		.map(|x| util::uuid::parse(&x).unwrap());

	assert_eq!(Some(game_id), redis_game_id);
}
