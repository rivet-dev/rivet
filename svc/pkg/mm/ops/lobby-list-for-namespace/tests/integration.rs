use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] mm_lobby_list_for_namespace {
		namespace_ids: vec![lobby_res.namespace_id.unwrap()],
	})
	.await
	.unwrap();

	assert!(
		res.namespaces
			.iter()
			.any(|lg| lg.lobby_ids.iter().any(|lobby_id| lobby_res
				.lobby_id
				.as_ref()
				.map_or(false, |res_lobby_id| lobby_id == res_lobby_id))),
		"lobby not listed"
	);
}
