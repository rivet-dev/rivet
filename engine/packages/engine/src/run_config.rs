use anyhow::*;
use rivet_service_manager::{RunConfigData, Service, ServiceKind};

pub fn config(_rivet_config: rivet_config::Config) -> Result<RunConfigData> {
	let services = vec![
		Service::new("api_peer", ServiceKind::ApiPeer, |config, pools| {
			Box::pin(rivet_api_peer::start(config, pools))
		}),
		Service::new("guard", ServiceKind::ApiPublic, |config, pools| {
			Box::pin(rivet_guard::start(config, pools))
		}),
		Service::new(
			"workflow_worker",
			ServiceKind::Standalone,
			|config, pools| Box::pin(rivet_workflow_worker::start(config, pools)),
		),
		Service::new("bootstrap", ServiceKind::Oneshot, |config, pools| {
			Box::pin(rivet_bootstrap::start(config, pools))
		}),
		// Service::new(
		// 	"pegboard_serverless",
		// 	// There should only be one of these, since it's auto-scaling requests
		// 	ServiceKind::Singleton,
		// 	|config, pools| Box::pin(pegboard_serverless::start(config, pools)),
		// ),
		// Core services
		Service::new("tracing_reconfigure", ServiceKind::Core, |config, pools| {
			Box::pin(rivet_tracing_reconfigure::start(config, pools))
		}),
		Service::new("cache_purge", ServiceKind::Core, |config, pools| {
			Box::pin(rivet_cache_purge::start(config, pools))
		}),
	];

	Ok(RunConfigData { services })
}
