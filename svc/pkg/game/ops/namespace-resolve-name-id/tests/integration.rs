use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let mut namespaces = Vec::<(Uuid, String)>::new();
	for _ in 0..4usize {
		let create_res = op!([ctx] faker_game_namespace {
			game_id: game_res.game_id,
			version_id: game_res.version_ids.first().cloned(),
			..Default::default()
		})
		.await
		.unwrap();

		let get_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![create_res.namespace_id.unwrap()],
		})
		.await
		.unwrap();
		let ns_data = get_res.namespaces.first().unwrap();

		namespaces.push((
			ns_data.namespace_id.unwrap().as_uuid(),
			ns_data.name_id.clone(),
		));
	}

	let mut all_name_ids = namespaces
		.iter()
		.map(|(_, name_id)| name_id.clone())
		.collect::<Vec<_>>();
	all_name_ids.push(util::faker::ident()); // Add bogus name ID
	let res = op!([ctx] game_namespace_resolve_name_id {
		game_id: game_res.game_id,
		name_ids: all_name_ids,
	})
	.await
	.unwrap();
	assert_eq!(namespaces.len(), res.namespaces.len());
}
