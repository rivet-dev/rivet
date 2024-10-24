use chirp_worker::prelude::*;

struct TestVersion {
	display_name: String,
}

impl TestVersion {
	fn generate(count: usize) -> Vec<TestVersion> {
		std::iter::repeat_with(|| TestVersion {
			display_name: util::faker::ident(),
		})
		.take(count)
		.collect()
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_versions = TestVersion::generate(8);

	let game_create_res = op!([ctx] faker_game {
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await
	.unwrap();

	let mut version_ids = Vec::<common::Uuid>::new();
	for version in &game_versions {
		let create_res = op!([ctx] game_version_create {
			game_id: game_create_res.game_id,
			display_name: version.display_name.clone(),
		})
		.await
		.unwrap();
		version_ids.push(create_res.version_id.unwrap());
	}

	let res = op!([ctx] game_version_list {
		game_ids: vec![game_create_res.game_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(2, res.games.len());
	let game_a = &res.games[0];
	let game_b = &res.games[1];
	assert_eq!(version_ids.len(), game_a.version_ids.len());
	assert_eq!(0, game_b.version_ids.len());
}
