mod common;

use chirp_worker::prelude::*;
use common::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn lobby_connectivity_http(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	// Create lobby
	let lobby_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_ready_complete(lobby_id) {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(setup.namespace_id.into()),
		lobby_group_id: Some(setup.lobby_group_id.into()),
		region_id: Some(setup.region_id.into()),
		create_ray_id: None,
		preemptively_created: false,

		creator_user_id: None,
		is_custom: false,
		publicity: None,
		lobby_config_json: None,
	})
	.await
	.unwrap();

	let ingress_hostname_http = format!(
		"http://{lobby_id}-test-http.lobby.{}.{}",
		setup.region.name_id,
		util::env::domain_job().unwrap(),
	);

	// Echo body
	let random_body = Uuid::new_v4().to_string();
	let client = reqwest::Client::new();
	let res = client
		.post(ingress_hostname_http)
		.body(random_body.clone())
		.send()
		.await
		.unwrap()
		.error_for_status()
		.unwrap();
	let res_text = res.text().await.unwrap();
	assert_eq!(random_body, res_text);
}
