use chirp_workflow::prelude::*;

#[workflow_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::dns() {
		return;
	}

	let datacenter_id = Uuid::new_v4();
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	// Sends `cluster::msg::datacenter_tls_issue`
	msg!([ctx] cluster::msg::datacenter_create(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		display_name: util::faker::ident(),

		provider: backend::cluster::Provider::Linode as i32,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: Vec::new(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
		prebakes_enabled: false,
	})
	.await
	.unwrap();

	let res = op!([ctx] cluster_datacenter_tls_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await
	.unwrap();
	let datacenter = res.datacenters.first().expect("datacenter not found");

	assert_eq!(
		backend::cluster::TlsState::Creating as i32,
		datacenter.state,
		"invalid initial state"
	);

	// Wait for tls cert
	loop {
		tokio::time::sleep(std::time::Duration::from_secs(5)).await;

		let res = op!([ctx] cluster_datacenter_tls_get {
			datacenter_ids: vec![datacenter_id.into()],
		})
		.await
		.unwrap();
		let datacenter = res.datacenters.first().expect("datacenter not found");

		if datacenter.gg_cert_pem.is_some() {
			break;
		}
	}
}
