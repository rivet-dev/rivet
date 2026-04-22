#[path = "common/api/mod.rs"]
mod api;
#[path = "common/ctx.rs"]
mod ctx;

use std::{collections::HashMap, future::Future, time::Duration};

use anyhow::{Context, Result};
use base64::Engine;
use gas::prelude::*;
use rivet_api_types::{
	actors::{
		import_export::{ExportActorIdsSelector, ExportRequest, ExportSelector, ImportRequest},
		kv_get, list,
	},
	namespaces::runner_configs::{RunnerConfig, RunnerConfigKind},
};

const RUNNER_NAME: &str = "import-export-runner";
const ACTOR_NAME: &str = "import-export-actor";
const KV_KEY: &[u8] = b"test-key";
const KV_VALUE: &[u8] = b"test-value";
const SQLITE_META_VALUE: &[u8] = b"sqlite-meta-payload";
const SQLITE_PAGE_VALUE: &[u8] = b"sqlite-page-payload";

#[test]
fn actor_import_export_round_trip_e2e() {
	run_test(30, |ctx| async move {
		let source = create_namespace(&ctx, "source").await?;
		let target = create_namespace(&ctx, "target").await?;

		upsert_normal_runner_config(ctx.leader_dc().guard_port(), &source.name, RUNNER_NAME)
			.await?;
		upsert_normal_runner_config(ctx.leader_dc().guard_port(), &target.name, RUNNER_NAME)
			.await?;

		let source_actor = create_sleeping_actor_with_kv(
			ctx.leader_dc(),
			&source,
			ACTOR_NAME,
			Some("round-trip-key".to_string()),
		)
		.await?;

		write_sqlite_v2_fixture(ctx.leader_dc(), source_actor.actor_id).await?;

		wait_for_actor(
			ctx.leader_dc().guard_port(),
			&source.name,
			ACTOR_NAME,
			source_actor.key.clone(),
		)
		.await?;

		let export = api::public::admin_actors_export(
			ctx.leader_dc().guard_port(),
			ExportRequest {
				namespace: source.name.clone(),
				selector: ExportSelector {
					all: None,
					actor_names: None,
					actor_ids: Some(ExportActorIdsSelector {
						ids: vec![source_actor.actor_id],
					}),
				},
			},
		)
		.await?;

		assert_eq!(export.actor_count, 1);

		let import = api::public::admin_actors_import(
			ctx.leader_dc().guard_port(),
			ImportRequest {
				target_namespace: target.name.clone(),
				archive_path: export.archive_path.clone(),
			},
		)
		.await?;

		assert_eq!(import.imported_actors, 1);
		assert_eq!(import.skipped_actors, 0);
		assert!(import.warnings.is_empty());

		let imported_actor = wait_for_actor(
			ctx.leader_dc().guard_port(),
			&target.name,
			ACTOR_NAME,
			source_actor.key.clone(),
		)
		.await?;

		assert_ne!(imported_actor.actor_id, source_actor.actor_id);
		assert_eq!(imported_actor.create_ts, source_actor.create_ts);
		assert!(imported_actor.start_ts.is_none());
		assert!(imported_actor.sleep_ts.is_some());

		let kv = api::public::actors_kv_get(
			ctx.leader_dc().guard_port(),
			kv_get::KvGetPath {
				actor_id: imported_actor.actor_id,
				key: base64::engine::general_purpose::STANDARD.encode(KV_KEY),
			},
			kv_get::KvGetQuery {
				namespace: target.name.clone(),
			},
		)
		.await?;

		assert_eq!(
			base64::engine::general_purpose::STANDARD
				.decode(kv.value)
				.context("decode imported kv value")?,
			KV_VALUE
		);

		assert_sqlite_v2_fixture(ctx.leader_dc(), imported_actor.actor_id).await?;

		tokio::fs::remove_dir_all(&export.archive_path)
			.await
			.with_context(|| format!("remove archive {}", export.archive_path))?;

		Ok(())
	});
}

#[test]
fn actor_import_export_skips_name_key_collisions_e2e() {
	run_test(30, |ctx| async move {
		let source = create_namespace(&ctx, "collision-source").await?;
		let target = create_namespace(&ctx, "collision-target").await?;

		upsert_normal_runner_config(ctx.leader_dc().guard_port(), &source.name, RUNNER_NAME)
			.await?;
		upsert_normal_runner_config(ctx.leader_dc().guard_port(), &target.name, RUNNER_NAME)
			.await?;

		let actor_key = Some("collision-key".to_string());
		let source_actor =
			create_sleeping_actor_with_kv(ctx.leader_dc(), &source, ACTOR_NAME, actor_key.clone())
				.await?;
		let existing_target_actor =
			create_sleeping_actor_with_kv(ctx.leader_dc(), &target, ACTOR_NAME, actor_key.clone())
				.await?;

		let export = api::public::admin_actors_export(
			ctx.leader_dc().guard_port(),
			ExportRequest {
				namespace: source.name.clone(),
				selector: ExportSelector {
					all: None,
					actor_names: None,
					actor_ids: Some(ExportActorIdsSelector {
						ids: vec![source_actor.actor_id],
					}),
				},
			},
		)
		.await?;

		let import = api::public::admin_actors_import(
			ctx.leader_dc().guard_port(),
			ImportRequest {
				target_namespace: target.name.clone(),
				archive_path: export.archive_path.clone(),
			},
		)
		.await?;

		assert_eq!(import.imported_actors, 0);
		assert_eq!(import.skipped_actors, 1);
		assert_eq!(import.warnings.len(), 1);

		let actors = list_matching_actors(
			ctx.leader_dc().guard_port(),
			&target.name,
			ACTOR_NAME,
			actor_key.clone(),
		)
		.await?;

		assert_eq!(actors.len(), 1);
		assert_eq!(actors[0].actor_id, existing_target_actor.actor_id);

		tokio::fs::remove_dir_all(&export.archive_path)
			.await
			.with_context(|| format!("remove archive {}", export.archive_path))?;

		Ok(())
	});
}

fn run_test<F, Fut>(timeout_secs: u64, test_fn: F)
where
	F: FnOnce(ctx::TestCtx) -> Fut,
	Fut: Future<Output = Result<()>>,
{
	let runtime = tokio::runtime::Runtime::new().expect("build tokio runtime");
	runtime.block_on(async move {
		let ctx = ctx::TestCtx::new_with_opts(ctx::TestOpts::new(1).with_timeout(timeout_secs))
			.await
			.expect("build test ctx");
		tokio::time::timeout(Duration::from_secs(timeout_secs), test_fn(ctx))
			.await
			.expect("test timed out")
			.expect("test failed");
	});
}

struct TestNamespace {
	name: String,
	id: rivet_util::Id,
}

async fn create_namespace(ctx: &ctx::TestCtx, prefix: &str) -> Result<TestNamespace> {
	let namespace_name = format!("{prefix}-{:04x}", rand::random::<u16>());
	let response = api::public::namespaces_create(
		ctx.leader_dc().guard_port(),
		rivet_api_peer::namespaces::CreateRequest {
			name: namespace_name,
			display_name: "Test Namespace".to_string(),
		},
	)
	.await?;

	Ok(TestNamespace {
		name: response.namespace.name,
		id: response.namespace.namespace_id,
	})
}

async fn upsert_normal_runner_config(port: u16, namespace: &str, runner_name: &str) -> Result<()> {
	let mut datacenters = HashMap::new();
	datacenters.insert(
		"dc-1".to_string(),
		RunnerConfig {
			kind: RunnerConfigKind::Normal {},
			metadata: None,
			drain_on_version_upgrade: true,
		},
	);

	api::public::runner_configs_upsert(
		port,
		rivet_api_peer::runner_configs::UpsertPath {
			runner_name: runner_name.to_string(),
		},
		rivet_api_peer::runner_configs::UpsertQuery {
			namespace: namespace.to_string(),
		},
		rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
	)
	.await?;

	Ok(())
}

async fn create_sleeping_actor_with_kv(
	dc: &ctx::TestDatacenter,
	namespace: &TestNamespace,
	name: &str,
	key: Option<String>,
) -> Result<rivet_types::actors::Actor> {
	let actor_id = rivet_util::Id::new_v1(dc.config.dc_label());
	let actor = dc
		.workflow_ctx
		.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: namespace.id,
			name: name.to_string(),
			key,
			runner_name_selector: RUNNER_NAME.to_string(),
			crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			input: None,
			start_immediately: false,
			create_ts: None,
			forward_request: false,
			datacenter_name: Some(
				dc.config
					.dc_name()
					.context("test dc missing name")?
					.to_string(),
			),
		})
		.await?
		.actor;

	let recipient = pegboard::actor_kv::Recipient {
		actor_id: actor.actor_id,
		namespace_id: namespace.id,
		name: actor.name.clone(),
	};
	pegboard::actor_kv::put(
		&*dc.workflow_ctx.udb().context("missing workflow db")?,
		&recipient,
		vec![KV_KEY.to_vec()],
		vec![KV_VALUE.to_vec()],
	)
	.await?;

	Ok(actor)
}

async fn wait_for_actor(
	port: u16,
	namespace: &str,
	name: &str,
	key: Option<String>,
) -> Result<rivet_types::actors::Actor> {
	let start = std::time::Instant::now();
	let timeout = Duration::from_secs(10);

	loop {
		let actors = list_matching_actors(port, namespace, name, key.clone()).await?;
		if let Some(actor) = actors.into_iter().next() {
			return Ok(actor);
		}

		if start.elapsed() >= timeout {
			anyhow::bail!("timed out waiting for actor {name} in namespace {namespace}");
		}

		tokio::time::sleep(Duration::from_millis(100)).await;
	}
}

async fn write_sqlite_v2_fixture(dc: &ctx::TestDatacenter, actor_id: rivet_util::Id) -> Result<()> {
	use sqlite_storage::keys::{meta_key, shard_key};

	let actor_str = actor_id.to_string();
	pegboard::actor_sqlite_v2::import_actor(
		&*dc.workflow_ctx.udb().context("missing workflow db")?,
		actor_id,
		vec![
			(
				strip_actor_prefix(&actor_str, meta_key(&actor_str)),
				SQLITE_META_VALUE.to_vec(),
			),
			(
				strip_actor_prefix(&actor_str, shard_key(&actor_str, 0)),
				SQLITE_PAGE_VALUE.to_vec(),
			),
		],
	)
	.await?;

	Ok(())
}

async fn assert_sqlite_v2_fixture(
	dc: &ctx::TestDatacenter,
	actor_id: rivet_util::Id,
) -> Result<()> {
	use sqlite_storage::keys::{meta_key, shard_key};

	let entries = pegboard::actor_sqlite_v2::export_actor(
		&*dc.workflow_ctx.udb().context("missing workflow db")?,
		actor_id,
	)
	.await?;
	let by_suffix: HashMap<Vec<u8>, Vec<u8>> = entries.into_iter().collect();
	let actor_str = actor_id.to_string();

	assert_eq!(
		by_suffix.get(&strip_actor_prefix(&actor_str, meta_key(&actor_str))),
		Some(&SQLITE_META_VALUE.to_vec()),
		"imported actor missing replayed sqlite META payload"
	);
	assert_eq!(
		by_suffix.get(&strip_actor_prefix(&actor_str, shard_key(&actor_str, 0))),
		Some(&SQLITE_PAGE_VALUE.to_vec()),
		"imported actor missing replayed sqlite SHARD payload"
	);

	Ok(())
}

fn strip_actor_prefix(actor_id: &str, full_key: Vec<u8>) -> Vec<u8> {
	let prefix = sqlite_storage::keys::actor_prefix(actor_id);
	full_key
		.strip_prefix(prefix.as_slice())
		.expect("sqlite key missing actor prefix")
		.to_vec()
}

async fn list_matching_actors(
	port: u16,
	namespace: &str,
	name: &str,
	key: Option<String>,
) -> Result<Vec<rivet_types::actors::Actor>> {
	Ok(api::public::actors_list(
		port,
		list::ListQuery {
			namespace: namespace.to_string(),
			name: Some(name.to_string()),
			key,
			actor_ids: None,
			actor_id: Vec::new(),
			include_destroyed: Some(false),
			limit: Some(10),
			cursor: None,
		},
	)
	.await?
	.actors)
}
