use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};

/// Route requests to the API service
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	_host: &str,
	path: &str,
) -> Result<Option<RoutingOutput>> {
	// Check target
	if target != "runner-ws" {
		return Ok(None);
	}

	let targets = vec![RouteTarget {
		actor_id: None,
		host: ctx.config().pegboard().lan_host().to_string(),
		port: ctx.config().pegboard().port(),
		path: path.to_owned(),
	}];

	return Ok(Some(RoutingOutput::Route(RouteConfig {
		targets,
		timeout: RoutingTimeout {
			routing_timeout: 10, // 10 seconds for API routing timeout
		},
	})));
}
