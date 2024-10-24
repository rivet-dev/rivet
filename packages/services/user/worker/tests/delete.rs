use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

// TODO: Verify user identity is deleted
#[worker_test]
async fn delete(ctx: TestCtx) {
	let user_id = Uuid::new_v4();
	tracing::info!(%user_id);

	msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
		user_id: Some(user_id.into()),
		namespace_id: None,
	})
	.await
	.unwrap();

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
			sqlx::query_as("SELECT delete_complete_ts FROM db_user.users WHERE user_id = $1")
				.bind(user_id)
				.fetch_one(&ctx.crdb().await.unwrap())
				.await
				.unwrap();
		assert!(delete_complete_ts.is_some(), "user not deleted");
	}
}
