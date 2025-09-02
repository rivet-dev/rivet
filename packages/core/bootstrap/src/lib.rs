use gas::prelude::*;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::from_pools(pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"bootstrap",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	tokio::try_join!(
		setup_epoxy_coordinator(&ctx),
		setup_epoxy_replica(&ctx),
		create_default_namespace(&ctx),
	)?;

	Ok(())
}

async fn setup_epoxy_coordinator(ctx: &StandaloneCtx) -> Result<()> {
	if !ctx.config().is_leader() {
		tracing::debug!("is not leader, skipping creating epoxy coordinator");
		return Ok(());
	}

	// Create coordinator if does not exist
	let workflow_id = ctx
		.workflow(epoxy::workflows::coordinator::Input {})
		.tag("replica", ctx.config().epoxy_replica_id())
		.unique()
		.dispatch()
		.await?;
	tracing::info!(%workflow_id, "created epoxy coordinator");

	// Check for reconfiguration.
	//
	// Do this on every startup in order to catch any possible config changes.
	//
	// This does not guarantee the config will change immediately since we can't guarantee that the
	// coordinator workflow is running on a node with the newest version of the config.
	ctx.signal(epoxy::workflows::coordinator::ReconfigureSignal {})
		.to_workflow_id(workflow_id)
		.send()
		.await?;
	tracing::info!(%workflow_id, "sent reconfigure message to epoxy coordinator");

	Ok(())
}

async fn setup_epoxy_replica(ctx: &StandaloneCtx) -> Result<()> {
	// Create replica if does not exist
	let workflow_id = ctx
		.workflow(epoxy::workflows::replica::Input {})
		.tag("replica", ctx.config().epoxy_replica_id())
		.unique()
		.dispatch()
		.await?;
	tracing::info!(%workflow_id, "created epoxy replica");

	Ok(())
}

async fn create_default_namespace(ctx: &StandaloneCtx) -> Result<()> {
	if !ctx.config().is_leader() {
		tracing::debug!("is not leader, skipping creating default namespace");
		return Ok(());
	}

	// Check if default namespace already exists
	let existing_namespace = ctx
		.op(namespace::ops::resolve_for_name_local::Input {
			name: "default".to_string(),
		})
		.await?;

	if existing_namespace.is_none() {
		// Create namespace
		let namespace_id = Id::new_v1(ctx.config().dc_label());
		let workflow_id = ctx
			.workflow(namespace::workflows::namespace::Input {
				namespace_id,
				name: "default".to_string(),
				display_name: "Default".to_string(),
			})
			.tag("namespace_id", namespace_id)
			.dispatch()
			.await?;
		tracing::info!(%workflow_id, %namespace_id, "created default namespace");
	} else {
		tracing::info!("default namespace already exists");
	}

	Ok(())
}
