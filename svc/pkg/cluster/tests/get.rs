use chirp_workflow::prelude::*;
use serde_json::json;

#[workflow_test]
async fn get(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();

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
			owner_team_id: None,
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
	let cluster = res.clusters.first().expect("cluster not found");

	assert_eq!(cluster_id, cluster.cluster_id);
}
