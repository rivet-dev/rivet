use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn prebake_install_complete(ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	let cluster_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let pool_type = backend::cluster::PoolType::Ats;
	let provider_datacenter_id = "us-southeast".to_string();

	msg!([ctx] cluster::msg::datacenter_create(datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(datacenter_id.into()),
		cluster_id: Some(cluster_id.into()),
		name_id: util::faker::ident(),
		display_name: util::faker::ident(),

		provider: backend::cluster::Provider::Linode as i32,
		provider_datacenter_id: provider_datacenter_id.clone(),
		provider_api_token: None,

		pools: Vec::new(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
		drain_timeout: 0,
	})
	.await
	.unwrap();

	msg!([ctx] linode::msg::prebake_provision(datacenter_id, pool_type as i32) {
		datacenter_id: Some(datacenter_id.into()),
		pool_type: pool_type as i32,
		provider_datacenter_id: provider_datacenter_id,
		tags: vec!["test".to_string()],
	})
	.await
	.unwrap();

	// Wait for server to have an ip
	let public_ip = loop {
		tokio::time::sleep(std::time::Duration::from_secs(5)).await;

		let row = sql_fetch_optional!(
			[ctx, (String,)]
			"
			SELECT public_ip
			FROM db_cluster.server_images_linode_misc
			WHERE
				provider = $1 AND
				install_hash = $2 AND
				datacenter_id = $3 AND
				pool_type = $4 AND
				public_ip IS NOT NULL
			",
			backend::cluster::Provider::Linode as i64,
			util_cluster::INSTALL_SCRIPT_HASH,
			datacenter_id,
			pool_type as i64,
		)
		.await
		.unwrap();

		if let Some((public_ip,)) = row {
			break public_ip;
		}
	};

	// Wait for install to complete
	let mut sub = subscribe!([ctx] cluster::msg::server_install_complete(public_ip))
		.await
		.unwrap();
	sub.next().await.unwrap();

	// Wait for server to have an image id
	loop {
		tokio::time::sleep(std::time::Duration::from_secs(2)).await;

		let (exists,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS (
				SELECT 1
				FROM db_cluster.server_images_linode_misc
				WHERE
					provider = $1 AND
					install_hash = $2 AND
					datacenter_id = $3 AND
					pool_type = $4 AND
					image_id IS NOT NULL
			)
			",
			backend::cluster::Provider::Linode as i64,
			util_cluster::INSTALL_SCRIPT_HASH,
			datacenter_id,
			pool_type as i64,
		)
		.await
		.unwrap();

		if exists {
			break;
		}
	}
}
