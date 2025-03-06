use anyhow::*;
use rivet_service_manager::{RunConfigData, Service, ServiceKind};

pub fn config(_rivet_config: rivet_config::Config) -> Result<RunConfigData> {
	let services = vec![
		// API
		Service::new(
			"api_edge_monolith_public",
			ServiceKind::ApiPublic,
			|config, pools| Box::pin(api_edge_monolith_public::start(config, pools)),
		),
		Service::new(
			"api_edge_monolith_edge",
			ServiceKind::ApiEdge,
			|config, pools| Box::pin(api_edge_monolith_edge::start(config, pools)),
		),
		Service::new("pegboard_ws", ServiceKind::ApiEdge, |config, pools| {
			Box::pin(pegboard_ws::start(config, pools))
		}),
		Service::new(
			"edge_monolith_workflow_worker",
			ServiceKind::Standalone,
			|config, pools| Box::pin(edge_monolith_workflow_worker::start(config, pools)),
		),
	];

	Ok(RunConfigData {
		services,
		sql_services: Vec::new(),
		s3_buckets: Vec::new(),
	})
}
