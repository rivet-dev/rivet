use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let user_id = Uuid::new_v4();
	let lobby_res = op!([ctx] faker_mm_lobby {
		creator_user_id: Some(user_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] mm_lobby_list_for_user_id {
		user_ids: vec![user_id.into()],
	})
	.await
	.unwrap();
	let user = res.users.first().unwrap();

	assert!(
		user.lobby_ids
			.iter()
			.any(|lobby_id| &lobby_res.lobby_id.unwrap() == lobby_id),
		"lobby not listed"
	);
}
