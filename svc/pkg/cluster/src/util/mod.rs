use chirp_workflow::prelude::*;
use cloudflare::framework as cf_framework;

use crate::types::PoolType;

pub mod metrics;
pub mod test;

// Use the hash of the server install script in the image variant so that if the install scripts are updated
// we won't be using the old image anymore
pub const INSTALL_SCRIPT_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/hash.txt"));

// TTL of the token written to prebake images. Prebake images are renewed before the token would expire
pub const SERVER_TOKEN_TTL: i64 = util::duration::days(30 * 6);

#[derive(thiserror::Error, Debug)]
#[error("cloudflare: {source}")]
struct CloudflareError {
	#[from]
	source: anyhow::Error,
}

// Cluster id for provisioning servers
pub fn default_cluster_id() -> Uuid {
	Uuid::nil()
}

pub fn server_name(provider_datacenter_id: &str, pool_type: PoolType, server_id: Uuid) -> String {
	let ns = util::env::namespace();
	let pool_type_str = match pool_type {
		PoolType::Job => "job",
		PoolType::Gg => "gg",
		PoolType::Ats => "ats",
	};

	format!("{ns}-{provider_datacenter_id}-{pool_type_str}-{server_id}",)
}

pub(crate) async fn cf_client() -> GlobalResult<cf_framework::async_api::Client> {
	// Create CF client
	let cf_token = util::env::read_secret(&["cloudflare", "terraform", "auth_token"]).await?;
	let client = cf_framework::async_api::Client::new(
		cf_framework::auth::Credentials::UserAuthToken { token: cf_token },
		Default::default(),
		cf_framework::Environment::Production,
	)
	.map_err(CloudflareError::from)?;

	Ok(client)
}
