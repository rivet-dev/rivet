use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let display_name = util::faker::display_name();
	let owner_user_id = Uuid::new_v4();

	let team_id = Uuid::new_v4();
	msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
		team_id: Some(team_id.into()),
		display_name: display_name.clone(),
		owner_user_id: Some(owner_user_id.into()),
	})
	.await
	.unwrap();

	let (sql_display_name,): (String,) =
		sqlx::query_as("SELECT display_name FROM teams WHERE team_id = $1")
			.bind(team_id)
			.fetch_one(&ctx.crdb("db-team").await.unwrap())
			.await
			.unwrap();
	assert_eq!(display_name, sql_display_name);
}

#[worker_test]
async fn duplicate_display_name(ctx: TestCtx) {
	let display_name = util::faker::display_name();
	let owner_user_id = Uuid::new_v4();

	let team_id = Uuid::new_v4();
	msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
		team_id: Some(team_id.into()),
		display_name: display_name.clone(),
		owner_user_id: Some(owner_user_id.into()),
	})
	.await
	.unwrap();

	// Create team with duplicate display name
	let team_id = Uuid::new_v4();
	let res = msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
		team_id: Some(team_id.into()),
		display_name: display_name.clone(),
		owner_user_id: Some(owner_user_id.into()),
	});

	// Wait 3 seconds before timing out, meaning the request didn't finish
	util::macros::select_with_timeout!([3 SEC] {
		_ = res => {
			panic!("duplicate named team created");
		}
	});
}
