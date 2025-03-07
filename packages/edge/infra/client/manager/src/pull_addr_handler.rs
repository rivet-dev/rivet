use std::time::{Duration, Instant};

use anyhow::*;
use pegboard_config::{Addresses, Client};
use service_discovery::ApiResponse;
use tokio::sync::RwLock;

/// Duration between pulling addresses again.
const PULL_INTERVAL: Duration = Duration::from_secs(3 * 60);

pub struct PullAddrHandler {
	last_pull: RwLock<Option<Instant>>,
	addresses: RwLock<Vec<String>>,
}

impl PullAddrHandler {
	pub fn new() -> Self {
		PullAddrHandler {
			last_pull: RwLock::new(None),
			addresses: RwLock::new(Vec::new()),
		}
	}

	pub async fn addresses(&self, client: &Client) -> Result<Vec<String>> {
		match &*client.images.pull_addresses() {
			Addresses::Dynamic { fetch_endpoint } => {
				let mut last_pull_guard = self.last_pull.write().await;

				if last_pull_guard
					.map(|x| x.elapsed() > PULL_INTERVAL)
					.unwrap_or(true)
				{
					let mut addr_guard = self.addresses.write().await;

					let mut addresses = reqwest::get(fetch_endpoint.clone())
						.await?
						.error_for_status()?
						.json::<ApiResponse>()
						.await?
						.servers
						.into_iter()
						.filter_map(|server| server.lan_ip)
						.map(|vlan_ip| format!("http://{vlan_ip}:8080"))
						.collect::<Vec<_>>();

					// Always sort the addresses so the list is deterministic
					addresses.sort();

					if addresses.is_empty() {
						tracing::warn!("empty list of pull addresses");
					}

					*addr_guard = addresses.clone();
					*last_pull_guard = Some(Instant::now());

					Ok(addresses)
				} else {
					Ok(self.addresses.read().await.clone())
				}
			}
			Addresses::Static(addresses) => Ok(addresses.clone()),
		}
	}
}
