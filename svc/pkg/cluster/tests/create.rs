use chirp_workflow::prelude::*;
use serde_json::json;

#[workflow_test]
async fn create(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();
	let owner_team_id = Uuid::new_v4();

	let mut sub = ctx
		.subscribe::<cluster::workflows::cluster::CreateComplete>(&json!({
			"cluster_id": cluster_id,
		}))
		.await
		.unwrap();

	ctx.dispatch_tagged_workflow(
		&json!({
			"cluster_id": cluster_id,
		}),
		cluster::workflows::cluster::Input {
			cluster_id,
			name_id: util::faker::ident(),
			owner_team_id: Some(owner_team_id),
		},
	)
	.await
	.unwrap();

	sub.next().await.unwrap();

	let res = ctx
		.op(cluster::ops::get::Input {
			cluster_ids: vec![cluster_id],
		})
		.await
		.unwrap();
	assert!(!res.clusters.is_empty(), "cluster not found");
}
