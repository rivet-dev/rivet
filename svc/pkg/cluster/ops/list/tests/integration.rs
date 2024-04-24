use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn list_single_cluster(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	let res = op!([ctx] cluster_list {}).await.unwrap();

	// The cluster should be in the list of all clusters
	let new_cluster_id = res
		.cluster_ids
		.into_iter()
		.find(|id| id.as_uuid() == cluster_id)
		.unwrap();

	assert_eq!(cluster_id, new_cluster_id.as_uuid());
}
