use anyhow::*;
use chirp_workflow::prelude::*;

/// Creates a login link for the hub.
pub async fn login_create(config: &rivet_config::Config, username: String) -> Result<String> {
	let pools = rivet_pools::Pools::new(config.clone()).await?;
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("rivet-cli");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools)
			.await
			.map_err(|err| anyhow!("{err}"))?,
		config.clone(),
		rivet_connection::Connection::new(client, pools, cache),
		"rivet-cli",
	)
	.await
	.map_err(|err| anyhow!("{err}"))?;

	let output = ctx
		.op(admin::ops::login_create::Input { username })
		.await
		.map_err(|err| anyhow!("{err}"))?;

	Ok(output.url)
}
