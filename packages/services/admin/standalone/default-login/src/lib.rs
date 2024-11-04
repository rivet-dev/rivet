use chirp_workflow::prelude::*;

// TODO: Should this token live forever or a shorter period of time?
// This token is printed on startup. It should be accessible if a dev checks the logs much later.
const TOKEN_TTL: i64 = util::duration::hours(24 * 7);

const DEFAULT_USERNAME: &'static str = "admin";

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("admin-default-login-url");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		config,
		rivet_connection::Connection::new(client, pools, cache),
		"admin-default-login-url",
	)
	.await?;

	let output = ctx
		.op(admin::ops::login_create::Input {
			username: DEFAULT_USERNAME.to_string(),
		})
		.await?;

	tracing::info!(url = ?output.url, "admin login url");

	Ok(())
}
