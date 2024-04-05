use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn prebake_install_complete(ctx: TestCtx) {
	if !util::feature::server_provision() {
		return;
	}

	let image_variant = util::faker::ident();
	let pool_type = backend::cluster::PoolType::Ats;

	msg!([ctx] linode::msg::prebake_provision(&image_variant) {
		variant: image_variant.clone(),
		provider_datacenter_id: "us-southeast".to_string(),
		pool_type: pool_type as i32,
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
				variant = $1 AND
				public_ip IS NOT NULL
			",
			&image_variant,
		)
		.await
		.unwrap();

		if let Some((public_ip,)) = row {
			break public_ip;
		}
	};

	tokio::time::sleep(std::time::Duration::from_secs(500)).await;

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
					variant = $1 AND
					image_id IS NOT NULL
			)
			",
			&image_variant,
		)
		.await
		.unwrap();

		if exists {
			break;
		}
	}

	todo!("");
}
