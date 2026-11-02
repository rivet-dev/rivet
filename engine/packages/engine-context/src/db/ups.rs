use anyhow::Result;
use rivet_topology::topology::Topology;
use rivet_topology::worker::WorkerIdentity;
use std::sync::Arc;
use universalpubsub::PubSub;

pub type UpsPool = PubSub;

pub async fn setup(
	env: Arc<dyn rivet_env::Env>,
	config: &rivet_config::Config,
	identity: &WorkerIdentity,
	topology: Arc<Topology>,
	http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
) -> Result<UpsPool> {
	let settings = config.pubsub();

	PubSub::new(
		env,
		identity.worker_id.clone(),
		identity.instance_id.into(),
		topology,
		http,
		settings.options(),
	)
}
