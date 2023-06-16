use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let reporter_user_id = Uuid::new_v4();
	let subject_user_id = Uuid::new_v4();
	let namespace_id = Uuid::new_v4();

	msg!([ctx] user_report::msg::create(subject_user_id) {
		reporter_user_id: Some(reporter_user_id.into()),
		subject_user_id: Some(subject_user_id.into()),
		namespace_id: Some(namespace_id.into()),
		reason: Some("big chungus".to_string()),
	})
	.await
	.unwrap();

	tokio::time::sleep(std::time::Duration::from_secs(2)).await;

	let (exists,): (bool,) =
		sqlx::query_as("SELECT EXISTS (SELECT 1 FROM user_reports WHERE reporter_user_id = $1)")
			.bind(reporter_user_id)
			.fetch_one(&ctx.crdb("db-user-report").await.unwrap())
			.await
			.unwrap();
	assert!(exists, "user report not created");
}
