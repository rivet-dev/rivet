use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let display_name = util::faker::display_name();
	let bio = util::faker::ident();
	let owner_user_id = Uuid::new_v4();
	let publicity = backend::team::Publicity::Open as i32;

	let team_id = Uuid::new_v4();
	msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
		team_id: Some(team_id.into()),
		display_name: "testname".to_owned(),
		owner_user_id: Some(owner_user_id.into())
	})
	.await
	.expect("team create");

	msg!([ctx] team::msg::profile_set(team_id) -> team::msg::profile_set_complete {
		team_id: Some(team_id.into()),
		display_name: Some(display_name.clone()),
		bio: Some(bio.clone()),
		publicity: Some(publicity)
	})
	.await
	.unwrap();

	let (sql_display_name, sql_bio, sql_publicity): (String, String, i64) =
		sqlx::query_as("SELECT display_name, bio, publicity FROM teams WHERE team_id = $1")
			.bind(team_id)
			.fetch_one(&ctx.crdb("db-team").await.unwrap())
			.await
			.unwrap();

	assert_eq!(display_name, sql_display_name);
	assert_eq!(bio, sql_bio);
	assert_eq!(publicity, sql_publicity as i32);
}
