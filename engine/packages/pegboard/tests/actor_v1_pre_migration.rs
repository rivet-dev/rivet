use std::time::Duration;

use gas::prelude::*;
use pegboard::workflows::actor::AllocationOverride;
use test_snapshot::SnapshotTestCtx;
use universaldb::prelude::*;

#[tokio::test]
async fn actor_v1_pre_migration() {
	let test_ctx = SnapshotTestCtx::from_snapshot_with_coordinator("pb-actor-v1-pre-migration")
		.await
		.unwrap();
	let ctx = test_ctx.get_ctx(test_ctx.leader_id);

	let existing_namespace = ctx
		.op(namespace::ops::resolve_for_name_local::Input {
			name: "default".to_string(),
		})
		.await
		.unwrap()
		.expect("default ns should exist");

	let actors_res = ctx
		.op(pegboard::ops::actor::list_for_ns::Input {
			namespace_id: existing_namespace.namespace_id,
			name: "test".to_string(),
			key: None,
			include_destroyed: true,
			created_before: None,
			limit: 1,
			fetch_error: false,
		})
		.await
		.unwrap();
	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.expect("actor should exist");

	ctx.signal(pegboard::workflows::actor::Wake {
		allocation_override: AllocationOverride::default(),
	})
	.to_workflow::<pegboard::workflows::actor::Workflow>()
	.tag("actor_id", actor.actor_id)
	.send()
	.await
	.unwrap();

	tokio::time::sleep(Duration::from_secs(3)).await;

	// Get workflow id
	let workflow_id = ctx
		.udb()
		.unwrap()
		.run(|tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());

			tx.read(
				&pegboard::keys::actor::WorkflowIdKey::new(actor.actor_id),
				Serializable,
			)
			.await
		})
		.await
		.unwrap();

	let wf = ctx
		.get_workflows(vec![workflow_id])
		.await
		.unwrap()
		.into_iter()
		.next()
		.expect("workflow should exist");

	assert!(!wf.is_dead());
}
