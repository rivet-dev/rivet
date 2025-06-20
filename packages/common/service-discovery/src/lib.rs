use std::{future::Future, sync::Arc, time::Duration};

use rand::Rng;
use reqwest::Client;
use tokio::{
	sync::{Mutex, RwLock},
	task::JoinHandle,
};
use url::Url;
use rivet_api::models::{ProvisionServer, ProvisionDatacentersGetServersResponse};

pub struct ServiceDiscovery {
	fetch_endpoint: Url,
	last: RwLock<Vec<ProvisionServer>>,
	handle: Mutex<Option<JoinHandle<()>>>,
}

impl ServiceDiscovery {
	pub fn new(fetch_endpoint: Url) -> Arc<Self> {
		Arc::new(ServiceDiscovery {
			fetch_endpoint,
			last: RwLock::new(Vec::new()),
			handle: Mutex::new(None),
		})
	}

	/// Starts a background tokio task that periodically fetches the endpoint and calls `cb`.
	pub fn start<F, Fut, E>(self: &Arc<Self>, cb: F)
	where
		F: Fn(Vec<ProvisionServer>) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Result<(), E>> + Send + 'static,
		E: std::fmt::Debug,
	{
		let mut guard = self.handle.try_lock().expect("already started");
		assert!(guard.is_none(), "already started");

		let self2 = self.clone();
		*guard = Some(tokio::task::spawn(async move {
			let client = Client::new();

			loop {
				let res = match self2.fetch_inner(&client).await {
					Ok(res) => res,
					Err(err) => {
						tracing::error!(?err, "fetch service discovery failed");
						continue;
					}
				};

				if let Err(err) = cb(res.servers.clone()).await {
					tracing::error!(?err, "service discovery callback failed");
				}

				{
					let mut guard = self2.last.write().await;
					*guard = res.servers;
				}

				let duration = Duration::from_secs(60)
					+ rand::thread_rng().gen_range(Duration::ZERO..Duration::from_secs(1));
				tokio::time::sleep(duration).await;
			}
		}));
	}

	/// Returns the last retrieved value without fetching.
	pub async fn get(&self) -> Vec<ProvisionServer> {
		self.last.read().await.clone()
	}

	/// Manually fetches the endpoint.
	pub async fn fetch(&self) -> Result<Vec<ProvisionServer>, reqwest::Error> {
		let client = Client::new();
		Ok(self.fetch_inner(&client).await?.servers)
	}

	async fn fetch_inner(&self, client: &Client) -> Result<ProvisionDatacentersGetServersResponse, reqwest::Error> {
		Ok(client
			.get(self.fetch_endpoint.clone())
			.send()
			.await?
			.error_for_status()?
			.json::<ProvisionDatacentersGetServersResponse>()
			.await?)
	}
}

impl Drop for ServiceDiscovery {
	// Stops the periodic handle if one exists.
	fn drop(&mut self) {
		if let Some(handle) = self.handle.try_lock().expect("should not be locked").take() {
			handle.abort();
		}
	}
}
