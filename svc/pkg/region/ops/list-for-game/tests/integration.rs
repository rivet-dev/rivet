use chirp_worker::prelude::*;
use serde_json::json;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let (cluster_id, datacenter_id) = create_dc(&ctx).await;

	let game_id = Uuid::new_v4();
	chirp_workflow::compat::signal(
		ctx.op_ctx(),
		cluster::workflows::cluster::GameLink { game_id },
	)
	.await
	.unwrap()
	.tag("cluster_id", cluster_id)
	.send()
	.await
	.unwrap();

	let regions_res = op!([ctx] region_list_for_game {
		game_ids: vec![game_id.into()],
	})
	.await
	.unwrap();

	assert_eq!(1, regions_res.region_ids.len(), "wrong number of regions");
	assert_eq!(
		datacenter_id,
		regions_res.region_ids.first().unwrap().as_uuid(),
		"wrong region id",
	);
}

async fn create_dc(ctx: &TestCtx) -> (Uuid, Uuid) {
	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	chirp_workflow::compat::workflow(
		ctx.op_ctx(),
		cluster::workflows::cluster::Input {
			cluster_id,
			name_id: util::faker::ident(),
			owner_team_id: None,
		},
	)
	.await
	.unwrap()
	.tag("cluster_id", cluster_id)
	.dispatch()
	.await
	.unwrap();

	let mut create_sub =
		chirp_workflow::compat::subscribe::<cluster::workflows::datacenter::CreateComplete, _>(
			ctx.op_ctx(),
			&json!({
				"datacenter_id": datacenter_id,
			}),
		)
		.await
		.unwrap();
	chirp_workflow::compat::signal(
		ctx.op_ctx(),
		cluster::workflows::cluster::DatacenterCreate {
			datacenter_id,
			name_id: util::faker::ident(),
			display_name: util::faker::ident(),

			provider: cluster::types::Provider::Linode,
			provider_datacenter_id: "us-southeast".to_string(),
			provider_api_token: None,

			pools: Vec::new(),

			build_delivery_method: cluster::types::BuildDeliveryMethod::TrafficServer,
			prebakes_enabled: false,
		},
	)
	.await
	.unwrap()
	.tag("cluster_id", cluster_id)
	.send()
	.await
	.unwrap();

	create_sub.next().await.unwrap();

	(cluster_id, datacenter_id)
}
