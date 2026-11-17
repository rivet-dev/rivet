use std::sync::Arc;

use anyhow::{Context, Result};
use reqwest::Client;
use rivet_outbound_guard::{GuardedResolver, Policy};
use tokio::sync::OnceCell;

static CLIENT: OnceCell<Client> = OnceCell::const_new();
static GUARDED_CLIENT: OnceCell<Client> = OnceCell::const_new();
static GUARDED_CLIENT_NO_TIMEOUT: OnceCell<Client> = OnceCell::const_new();
static OUTBOUND_POLICY: OnceCell<Arc<Policy>> = OnceCell::const_new();
static CLIENT_USER_AGENT: &str = concat!("RivetEngine/", env!("CARGO_PKG_VERSION"));

/// Client for trusted destinations inside the engine network, such as peer datacenters and epoxy
/// replicas.
///
/// Never use this for a URL that came from user configuration. Those go through
/// [`guarded_client`], which restricts what the request can reach.
pub async fn client() -> Result<Client, reqwest::Error> {
	CLIENT
		.get_or_try_init(|| async {
			Client::builder()
				.user_agent(CLIENT_USER_AGENT)
				.timeout(std::time::Duration::from_secs(30))
				.build()
		})
		.await
		.cloned()
}

/// Client for destinations that come from user configuration, such as serverless runner URLs.
///
/// The `outbound` security policy is enforced at DNS resolution time and on every redirect, so
/// these requests cannot be steered at services only reachable from inside the engine network.
pub async fn guarded_client(config: &rivet_config::Config) -> Result<Client> {
	GUARDED_CLIENT
		.get_or_try_init(|| async {
			guarded_client_builder(config)
				.await?
				.timeout(std::time::Duration::from_secs(30))
				.build()
				.context("failed building guarded client")
		})
		.await
		.cloned()
}

/// Same as [`guarded_client`] but without a request timeout, for long-lived streaming requests
/// such as the serverless SSE connection.
///
/// Uses HTTP/1.1 so each stream holds its own connection. Over HTTP/2 every stream to a host
/// shares one connection, and once the server's `SETTINGS_MAX_CONCURRENT_STREAMS` is reached
/// (100 on Cloud Run) new requests wait in the client with no timeout until a long-lived stream
/// ends.
pub async fn guarded_client_no_timeout(config: &rivet_config::Config) -> Result<Client> {
	GUARDED_CLIENT_NO_TIMEOUT
		.get_or_try_init(|| async {
			guarded_client_builder(config)
				.await?
				.http1_only()
				.build()
				.context("failed building guarded client")
		})
		.await
		.cloned()
}

async fn guarded_client_builder(config: &rivet_config::Config) -> Result<reqwest::ClientBuilder> {
	let policy = outbound_policy(config).await?;

	Ok(Client::builder()
		.user_agent(CLIENT_USER_AGENT)
		.dns_resolver(Arc::new(GuardedResolver::new(policy.clone())))
		.redirect(rivet_outbound_guard::redirect_policy(policy)))
}

/// The destination policy applied to every request to a user-configured URL.
///
/// Callers use this to reject a URL at the point it is submitted, before it is ever stored. The
/// guarded clients apply the same policy again when they connect.
pub async fn outbound_policy(config: &rivet_config::Config) -> Result<Arc<Policy>> {
	OUTBOUND_POLICY
		.get_or_try_init(|| async { Policy::from_config(config).map(Arc::new) })
		.await
		.cloned()
}
