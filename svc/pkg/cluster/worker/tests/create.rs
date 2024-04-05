use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn create(ctx: TestCtx) {
	let cluster_id = Uuid::new_v4();
	let owner_team_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		owner_team_id: Some(owner_team_id.into()),
		name_id: util::faker::ident(),
	})
	.await
	.unwrap();
}
