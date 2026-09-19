use anyhow::*;
use rivet_service_manager::{RunConfigData, Service, ServiceKind};

pub fn config(_rivet_config: rivet_config::Config) -> Result<RunConfigData> {
	let services = vec![
		Service::new(
			"api_peer",
			ServiceKind::ApiPeer,
			|config, pools| Box::pin(rivet_api_peer::start(config, pools)),
			false,
		),
		Service::new(
			"guard",
			ServiceKind::ApiPublic,
			|config, pools| Box::pin(rivet_guard::start(config, pools)),
			true,
		),
		Service::new(
			"workflow_worker",
			ServiceKind::Standalone,
			|config, pools| Box::pin(rivet_workflow_worker::start(config, pools)),
			true,
		),
		Service::new(
			"pegboard_outbound",
			ServiceKind::Standalone,
			|config, pools| Box::pin(pegboard_outbound::start(config, pools)),
			true,
		),
		Service::new(
			"bootstrap",
			ServiceKind::Oneshot,
			|config, pools| Box::pin(rivet_bootstrap::start(config, pools)),
			false,
		),
		// Core services
		Service::new(
			"dynamic_config",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_dynamic_config::start(config, pools)),
			false,
		),
		Service::new(
			"version_management",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_version_management::start(config, pools)),
			false,
		),
		Service::new(
			"epoxy_protocol_version",
			ServiceKind::Core,
			|config, pools| Box::pin(epoxy::protocol_version::start(config, pools)),
			false,
		),
		Service::new(
			"cache_purge",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_cache_purge::start(config, pools)),
			false,
		),
		Service::new(
			"profiling",
			ServiceKind::Core,
			|config, pools| Box::pin(rivet_profiling::start(config, pools)),
			false,
		),
	];

	Ok(RunConfigData { services })
}
