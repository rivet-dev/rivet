use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

pub struct Setup {
	pub server_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub pool_type: backend::cluster::PoolType,
	pub drain_timeout: u64,
}

pub struct SetupRes {
	pub pools: Vec<backend::cluster::Pool>,
	pub provider: backend::cluster::Provider,
}

pub async fn setup(ctx: &TestCtx, opts: Setup) -> SetupRes {
	let pools = vec![backend::cluster::Pool {
		pool_type: opts.pool_type as i32,
		hardware: vec![backend::cluster::Hardware {
			provider_hardware: util_cluster::test::LINODE_HARDWARE.to_string(),
		}],
		desired_count: 0,
		max_count: 0,
		drain_timeout: opts.drain_timeout,
	}];
	let provider = backend::cluster::Provider::Linode;

	msg!([ctx] cluster::msg::create(opts.cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(opts.cluster_id.into()),
		name_id: util::faker::ident(),
		owner_team_id: None,
	})
	.await
	.unwrap();

	msg!([ctx] cluster::msg::datacenter_create(opts.datacenter_id) -> cluster::msg::datacenter_scale {
		datacenter_id: Some(opts.datacenter_id.into()),
		cluster_id: Some(opts.cluster_id.into()),
		name_id: util::faker::ident(),
		display_name: util::faker::ident(),

		provider: provider as i32,
		provider_datacenter_id: "us-southeast".to_string(),
		provider_api_token: None,

		pools: pools.clone(),

		build_delivery_method: backend::cluster::BuildDeliveryMethod::TrafficServer as i32,
	})
	.await
	.unwrap();

	// Write new server to db
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.servers (
			server_id,
			datacenter_id,
			pool_type,
			create_ts
		)
		VALUES ($1, $2, $3, $4)
		",
		opts.server_id,
		opts.datacenter_id,
		opts.pool_type as i64,
		util::timestamp::now(),
	)
	.await
	.unwrap();

	SetupRes { pools, provider }
}
