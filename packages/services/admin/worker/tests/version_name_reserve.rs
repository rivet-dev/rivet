use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn version_name_reserve(ctx: TestCtx) {
	let now = chrono::Utc::now();
	let date_prefix = now.format("%Y.%m").to_string();

	let game_id = Uuid::new_v4();
	let request_id = Uuid::new_v4();

	for i in 1..20 {
		let res = msg!([ctx] cloud::msg::version_name_reserve(game_id, request_id) -> cloud::msg::version_name_reserve_complete {
			game_id: Some(game_id.into()),
			request_id: Some(request_id.into()),
		})
		.await
		.unwrap();

		assert_eq!(format!("{} ({})", date_prefix, i), res.version_display_name);
	}
}
