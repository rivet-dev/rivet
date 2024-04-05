use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn prebake_provision(ctx: TestCtx) {
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
	loop {
		tokio::time::sleep(std::time::Duration::from_secs(5)).await;

		let (exists,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS (
				SELECT 1
				FROM db_cluster.server_images_linode_misc
				WHERE
					variant = $1 AND
					public_ip IS NOT NULL
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
}
