use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();
	let name_id = util::faker::ident();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		name_id: name_id.clone(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	let res = op!([ctx] cluster_resolve_for_name_id {
		name_ids: vec![name_id],
	})
	.await
	.unwrap();

	let cluster = res.clusters.first().expect("cluster not found");
	assert_eq!(
		cluster_id,
		cluster.cluster_id.unwrap().as_uuid(),
		"wrong cluster returned"
	);
}
