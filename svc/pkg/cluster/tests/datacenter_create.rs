use chirp_workflow::prelude::*;
use serde_json::json;

#[workflow_test]
async fn datacenter_create(ctx: TestCtx) {
	let datacenter_id = Uuid::new_v4();
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

	ctx.tagged_signal(
		&json!({
			"cluster_id": cluster_id,
		}),
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
	.unwrap();

	// Check if tls record exists
	let (exists,) = sql_fetch_one!(
		[ctx, (bool,)]
		"
		SELECT EXISTS (
			SELECT 1
			FROM db_cluster.datacenter_tls
			WHERE datacenter_id = $1
		)
		",
		datacenter_id,
	)
	.await
	.unwrap();

	assert!(exists, "no tls record");
}
