use std::collections::HashSet;

use chirp_worker::prelude::*;

struct TestNamespace {
	display_name: String,
	name_id: String,
}

impl TestNamespace {
	fn generate(count: usize) -> Vec<TestNamespace> {
		std::iter::repeat_with(|| TestNamespace {
			display_name: util::faker::display_name(),
			name_id: util::faker::ident(),
		})
		.take(count)
		.collect()
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_namespaces = TestNamespace::generate(8);

	let game_create_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	let version_create_res = op!([ctx] game_version_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::ident(),
	})
	.await
	.unwrap();

	let mut namespace_ids = HashSet::<Uuid>::new();
	for namespace in &game_namespaces {
		let create_res = op!([ctx] game_namespace_create {
			game_id: game_create_res.game_id,
			display_name: namespace.display_name.clone(),
			version_id: version_create_res.version_id,
			name_id: namespace.name_id.clone(),
		})
		.await
		.unwrap();
		namespace_ids.insert(create_res.namespace_id.unwrap().as_uuid());
	}

	let res = op!([ctx] game_namespace_list {
		game_ids: vec![game_create_res.game_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(2, res.games.len());
	let game_a = &res.games[0];
	let game_b = &res.games[1];
	assert_eq!(namespace_ids.len(), game_a.namespace_ids.len());
	assert_eq!(0, game_b.namespace_ids.len());
}
