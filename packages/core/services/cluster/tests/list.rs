use chirp_workflow::prelude::*;
use serde_json::json;

#[workflow_test]
async fn list_single_cluster(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();

	let mut sub = ctx
		.subscribe::<cluster::workflows::cluster::CreateComplete>(&json!({
			"cluster_id": cluster_id,
		}))
		.await
		.unwrap();

	ctx.workflow(cluster::workflows::cluster::Input {
		cluster_id,
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.tag("cluster_id", cluster_id)
	.dispatch()
	.await
	.unwrap();

	sub.next().await.unwrap();

	let res = ctx.op(cluster::ops::list::Input {}).await.unwrap();

	// The cluster should be in the list of all clusters
	res.cluster_ids
		.into_iter()
		.find(|id| id == &cluster_id)
		.unwrap();
}
