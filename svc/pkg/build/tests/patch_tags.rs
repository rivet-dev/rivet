use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::backend;
use std::collections::HashMap;

#[workflow_test]
async fn patch_tags_works(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();
	let build_id = build_res.build_id.as_ref().unwrap().as_uuid();

	ctx.op(build::ops::patch_tags::Input {
		build_id,
		tags: HashMap::from([("tag1".to_string(), "value1".to_string())]),
		exclusive_tags: None,
	})
	.await
	.unwrap();

	let build = ctx
		.op(build::ops::get::Input {
			build_ids: vec![build_id],
		})
		.await
		.unwrap()
		.builds;
	assert_eq!(
		build[0].tags,
		HashMap::from([("tag1".to_string(), "value1".to_string())])
	);
}

#[workflow_test]
async fn patch_tags_overlapping_keys_error(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();
	let build_id = build_res.build_id.as_ref().unwrap().as_uuid();

	ctx.op(build::ops::patch_tags::Input {
		build_id,
		tags: HashMap::from([("tag1".to_string(), "value1".to_string())]),
		exclusive_tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
	})
	.await
	.unwrap_err();
}

#[workflow_test]
async fn patch_tags_exclusive_removes_tag_on_other_build(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_a_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();
	let build_a_id = build_a_res.build_id.as_ref().unwrap().as_uuid();

	let build_b_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();
	let build_b_id = build_b_res.build_id.as_ref().unwrap().as_uuid();

	ctx.op(build::ops::patch_tags::Input {
		build_id: build_a_id,
		tags: HashMap::from([
			("tag1".to_string(), "value1".to_string()),
			("tag2".to_string(), "value2".to_string()),
			("tag3".to_string(), "value3".to_string()),
		]),
		exclusive_tags: None,
	})
	.await
	.unwrap();

	ctx.op(build::ops::patch_tags::Input {
		build_id: build_b_id,
		tags: HashMap::from([("tag1".to_string(), "value2".to_string())]),
		exclusive_tags: Some(vec!["tag1".to_string()]),
	})
	.await
	.unwrap();

	let builds = ctx
		.op(build::ops::get::Input {
			build_ids: vec![build_a_id, build_b_id],
		})
		.await
		.unwrap()
		.builds;

	let build_a = builds.iter().find(|b| b.build_id == build_a_id).unwrap();
	let build_b = builds.iter().find(|b| b.build_id == build_b_id).unwrap();

	assert_eq!(
		build_a.tags,
		HashMap::from([
			("tag2".to_string(), "value2".to_string()),
			("tag3".to_string(), "value3".to_string()),
		])
	);

	assert_eq!(
		build_b.tags,
		HashMap::from([("tag1".to_string(), "value2".to_string())])
	);
}
