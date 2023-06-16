use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	struct TestNamespace {
		namespace_id: Option<Uuid>,
		name_id: String,
		display_name: String,
	}

	let mut test_namespaces = std::iter::repeat_with(|| TestNamespace {
		namespace_id: None,
		name_id: util::faker::ident(),
		display_name: util::faker::display_name(),
	})
	.take(8)
	.collect::<Vec<_>>();

	let game_create_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_create_res = op!([ctx] game_version_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::display_name(),
	})
	.await
	.unwrap();

	for namespace in &mut test_namespaces {
		let namespace_create_res = op!([ctx] game_namespace_create {
			game_id: game_create_res.game_id,
			display_name: namespace.display_name.clone(),
			name_id: namespace.name_id.clone(),
			version_id: version_create_res.version_id,
		})
		.await
		.unwrap();
		namespace.namespace_id = namespace_create_res.namespace_id.map(|id| id.as_uuid());
	}

	let res = op!([ctx] game_namespace_get {
		namespace_ids: test_namespaces
			.iter()
			.map(|n| n.namespace_id.unwrap().into())
			.collect(),
	})
	.await
	.unwrap();

	test_namespaces.sort_by(|a, b| a.display_name.cmp(&b.display_name));
	assert_eq!(test_namespaces.len(), res.namespaces.len());
	for (a, b) in test_namespaces.iter().zip(res.namespaces.iter()) {
		assert_eq!(a.namespace_id.unwrap(), b.namespace_id.unwrap().as_uuid());
	}
}
