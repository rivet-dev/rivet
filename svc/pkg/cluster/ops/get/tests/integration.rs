use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	let res = op!([ctx] cluster_get {
		cluster_ids: vec![cluster_id.into()],
	})
	.await
	.unwrap();
	let cluster = res.clusters.first().expect("cluster not found");

	assert_eq!(cluster_id, cluster.cluster_id.unwrap().as_uuid());
}
