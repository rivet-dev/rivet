use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_res.lobby_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(1, res.lobbies.len());
}

#[worker_test]
async fn fetch_batch(ctx: TestCtx) {
	let lobby_ids = std::iter::repeat_with(|| Into::<common::Uuid>::into(Uuid::new_v4()))
		.take(1024)
		.collect::<Vec<_>>();

	op!([ctx] mm_lobby_get {
		lobby_ids: lobby_ids,
		include_stopped: true,
	})
	.await
	.unwrap();
}
