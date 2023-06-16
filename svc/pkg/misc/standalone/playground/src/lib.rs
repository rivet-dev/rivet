use rivet_operation::prelude::*;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

mod empty_nodes;
mod orphaned_jobs;
mod orphaned_lobbies;
mod oversized_teams;
mod pull_game;
mod push_game;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(
	shared_client: chirp_client::SharedClientHandle,
	pools: rivet_pools::Pools,
	cache: Arc<rivet_cache::CacheInner>,
) -> GlobalResult<()> {
	let client = shared_client.wrap_new("playground");
	let ctx = OperationContext::new(
		"playground".into(),
		Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	// orphaned_jobs::run(pools, ctx).await?;
	// empty_nodes::run(pools, ctx).await?;
	// orphaned_lobbies::run(pools, ctx).await?;
	// oversized_teams::run(pools, ctx).await?;

	// pull_game::run(
	// 	pools,
	// 	ctx,
	// 	util::uuid::parse("5013a065-07ea-4063-b8c2-35f0b88c8171")?,
	// )
	// .await?;

	// push_game::run(
	// 	pools,
	// 	ctx,
	// 	util::uuid::parse("bcea2a9e-127f-4178-b3da-5758a64f0d9a")?,
	// 	Path::new("/tmp/rivet-d5655dd9-5acd-4eb0-8e01-47b2edb8ffb0"),
	// )
	// .await?;

	// Don't exit since Nomad will remove the logs
	tracing::info!("complete");

	tokio::signal::ctrl_c().await?;
	tracing::info!("received ctrl c");

	Ok(())
}
