use depot_client::Client;
use rivet_topology::topology::Topology;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

pub(crate) struct DepotPool {
	pub(crate) ready: tokio::sync::Notify,
	pub(crate) volumes: OnceLock<depot_client::volumes::VolumeClient>,
	pub(crate) service: Mutex<Option<crate::services::depot_client::Service>>,
	pub(crate) client: Client,
	pub(crate) topology: Arc<Topology>,
}
