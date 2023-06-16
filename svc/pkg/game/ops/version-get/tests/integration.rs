use chirp_worker::prelude::*;

struct TestVersion {
	display_name: String,
}

impl TestVersion {
	fn generate(count: usize) -> Vec<TestVersion> {
		std::iter::repeat_with(|| TestVersion {
			display_name: util::faker::display_name(),
		})
		.take(count)
		.collect()
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_versions = TestVersion::generate(8);

	let game_create_res = op!([ctx] faker_game {
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

	let res = op!([ctx] game_version_get {
		version_ids: version_ids.clone(),
	})
	.await
	.unwrap();

	for (version_id, version) in version_ids.iter().zip(game_versions.iter()) {
		let version_res = res
			.versions
			.iter()
			.find(|x| x.version_id == Some(Into::<common::Uuid>::into(*version_id)))
			.unwrap();
		assert_eq!(version.display_name, version_res.display_name);
	}
}
