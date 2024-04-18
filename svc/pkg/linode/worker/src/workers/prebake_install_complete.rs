use std::net::IpAddr;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use util_linode::api;

#[derive(sqlx::FromRow)]
struct PrebakeServer {
	install_hash: String,
	datacenter_id: Uuid,
	pool_type: i64,

	linode_id: i64,
	disk_id: i64,
}

#[worker(name = "linode-prebake-install-complete")]
async fn worker(
	ctx: &OperationContext<linode::msg::prebake_install_complete::Message>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();
	let public_ip = unwrap!(ctx.public_ip.parse::<IpAddr>(), "invalid public ip");

	let prebake_server = sql_fetch_one!(
		[ctx, PrebakeServer]
		"
		SELECT
			install_hash, datacenter_id, pool_type, linode_id, disk_id
		FROM db_cluster.server_images_linode
		WHERE
			public_ip = $1
		",
		public_ip,
	)
	.await?;

	let datacenter_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());

	// Build HTTP client
	let client = util_linode::Client::new(datacenter.provider_api_token.clone()).await?;

	// Shut down server before creating custom image
	api::shut_down(&client, prebake_server.linode_id).await?;

	// NOTE: Linode imposes a restriction of 50 characters on custom image labels, so unfortunately we cannot
	// use the image variant as the name. All we need from the label is for it to be unique. Keep in mind that
	// the UUID and hyphen take 37 characters, leaving us with 13 for the namespace name
	let name = format!("{}-{}", util::env::namespace(), Uuid::new_v4());

	let create_image_res = api::create_custom_image(&client, &name, prebake_server.disk_id).await?;

	// Write image id
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.server_images_linode
		SET image_id = $4
		WHERE
			install_hash = $1 AND
			datacenter_id = $2 AND
			pool_type = $3
		",
		prebake_server.install_hash,
		prebake_server.datacenter_id,
		prebake_server.pool_type,
		create_image_res.id,
	)
	.await?;

	Ok(())
}
