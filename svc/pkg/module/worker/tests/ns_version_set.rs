use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use std::collections::HashMap;

#[worker_test]
async fn ns_version_set(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let crdb = ctx.crdb().await.unwrap();

	let module_id = Uuid::new_v4();

	// Create game
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap().as_uuid();
	let namespace_id = game_res.namespace_ids.first().unwrap().as_uuid();

	// Create module
	{
		msg!([ctx] module::msg::create(module_id) -> module::msg::create_complete {
			module_id: Some(module_id.into()),
			name_id: "test".into(),
			team_id: Some(Uuid::new_v4().into()),
			creator_user_id: None,
		})
		.await
		.unwrap();
	}

	// Create fake versions
	let mv_a = create_module_version(
		&ctx,
		module_id,
		"ghcr.io/rivet-gg/rivet-module-hello-world:0.0.1",
		(0, 0, 1),
	)
	.await;
	let mv_b = create_module_version(
		&ctx,
		module_id,
		"ghcr.io/rivet-gg/rivet-module-hello-world:0.0.2",
		(0, 0, 2),
	)
	.await;
	let mv_c = create_module_version(
		&ctx,
		module_id,
		"ghcr.io/rivet-gg/rivet-module-hello-world:0.0.3",
		(0, 0, 3),
	)
	.await;

	// Deploy initial version
	let versions_a = {
		let mut create_sub = subscribe!([ctx] module::msg::instance_create("*"))
			.await
			.unwrap();
		deploy_game_version(
			&ctx,
			game_id,
			namespace_id,
			vec![("module-a".into(), mv_a), ("module-b".into(), mv_b)],
		)
		.await;
		create_sub.next().await.unwrap();
		create_sub.next().await.unwrap();

		// Check instances created
		let versions = get_namespace_module_version(&crdb, namespace_id).await;
		assert_eq!(2, versions.len());
		assert_eq!(mv_a, versions["module-a"].module_version_id);
		assert_eq!(mv_b, versions["module-b"].module_version_id);

		versions
	};

	// Deploy new version
	let versions_b = {
		let mut update_sub =
			subscribe!([ctx] module::msg::instance_version_set(versions_a["module-a"].instance_id))
				.await
				.unwrap();
		deploy_game_version(
			&ctx,
			game_id,
			namespace_id,
			vec![("module-a".into(), mv_a), ("module-b".into(), mv_c)],
		)
		.await;
		update_sub.next().await.unwrap();

		// Check instances updated & using same instances
		let versions = get_namespace_module_version(&crdb, namespace_id).await;
		assert_eq!(2, versions.len());
		assert_eq!(mv_a, versions["module-a"].module_version_id);
		assert_eq!(mv_c, versions["module-b"].module_version_id);
		assert_eq!(
			versions_a["module-a"].instance_id,
			versions["module-a"].instance_id
		);
		assert_eq!(
			versions_a["module-b"].instance_id,
			versions["module-b"].instance_id
		);

		versions
	};

	// Remove version
	let versions_c = {
		let mut destroy_sub =
			subscribe!([ctx] module::msg::instance_destroy(versions_b["module-a"].instance_id))
				.await
				.unwrap();
		deploy_game_version(&ctx, game_id, namespace_id, vec![("module-b".into(), mv_c)]).await;
		destroy_sub.next().await.unwrap();

		// Check instance removed
		let versions = get_namespace_module_version(&crdb, namespace_id).await;
		assert_eq!(1, versions.len());
		assert_eq!(mv_c, versions["module-b"].module_version_id);
		assert_eq!(
			versions_b["module-b"].instance_id,
			versions["module-b"].instance_id
		);

		versions
	};

	// Create module A again
	{
		let mut create_sub = subscribe!([ctx] module::msg::instance_create("*"))
			.await
			.unwrap();
		deploy_game_version(
			&ctx,
			game_id,
			namespace_id,
			vec![("module-a".into(), mv_a), ("module-b".into(), mv_c)],
		)
		.await;
		create_sub.next().await.unwrap();

		// Check new instance created for module_a
		let versions = get_namespace_module_version(&crdb, namespace_id).await;
		assert_eq!(2, versions.len());
		assert_eq!(mv_a, versions["module-a"].module_version_id);
		assert_eq!(mv_c, versions["module-b"].module_version_id);
		assert_ne!(
			versions_a["module-a"].instance_id,
			versions["module-a"].instance_id
		);
		assert_eq!(
			versions_c["module-b"].instance_id,
			versions["module-b"].instance_id
		);
	}
}

async fn create_module_version(
	ctx: &TestCtx,
	module_id: Uuid,
	tag: &str,
	(major, minor, patch): (u64, u64, u64),
) -> Uuid {
	// Create module version
	let module_version_id = Uuid::new_v4();
	msg!([ctx] module::msg::version_create(module_version_id) -> module::msg::version_create_complete {
		version_id: Some(module_version_id.into()),
		module_id: Some(module_id.into()),
		creator_user_id: None,

		major: major,
		minor: minor,
		patch: patch,

		scripts: vec![],

		image: Some(module::msg::version_create::message::Image::Docker(module::msg::version_create::message::Docker {
			image_tag: tag.to_string(),
		})),
	})
	.await
	.unwrap();

	module_version_id
}

async fn deploy_game_version(
	ctx: &TestCtx,
	game_id: Uuid,
	namespace_id: Uuid,
	dependencies: Vec<(String, Uuid)>,
) {
	// Deploy version
	let mut complete_sub = subscribe!([ctx] module::msg::ns_version_set_complete(namespace_id))
		.await
		.unwrap();
	op!([ctx] faker_game_version {
		game_id: Some(game_id.into()),
		deploy_to_namespace_id: Some(namespace_id.into()),
		override_module_config: Some(faker::game_version::request::OverrideModuleConfig {
			config: Some(backend::module::GameVersionConfig {
				dependencies: dependencies.into_iter().map(|(key, module_version_id)| {
					backend::module::game_version_config::Dependency {
						key,
						module_version_id: Some(module_version_id.into()),
					}
				}).collect(),
			})
		}),
		..Default::default()
	})
	.await
	.unwrap();
	complete_sub.next().await.unwrap();
}

#[derive(Debug)]
struct NamespaceModule {
	instance_id: Uuid,
	module_version_id: Uuid,
}

async fn get_namespace_module_version(
	crdb: &CrdbPool,
	namespace_id: Uuid,
) -> HashMap<String, NamespaceModule> {
	let versions = sqlx::query_as::<_, (String, Uuid, Uuid)>(indoc!(
		"
		SELECT ni.key, ni.instance_id, i.version_id
		FROM db_module.namespace_instances AS ni
		INNER JOIN instances AS i ON i.instance_id = ni.instance_id
		WHERE ni.namespace_id = $1
		"
	))
	.bind(namespace_id)
	.fetch_all(crdb)
	.await
	.unwrap()
	.into_iter()
	.map(|(key, instance_id, module_version_id)| {
		(
			key,
			NamespaceModule {
				instance_id,
				module_version_id,
			},
		)
	})
	.collect::<HashMap<_, _>>();

	tracing::info!(?versions, "namespace module versions");

	versions
}
