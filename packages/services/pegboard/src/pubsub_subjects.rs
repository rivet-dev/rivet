use gas::prelude::*;

pub struct RunnerReceiverSubject {
	runner_id: Id,
}

impl RunnerReceiverSubject {
	pub fn new(runner_id: Id) -> Self {
		Self { runner_id }
	}
}

impl std::fmt::Display for RunnerReceiverSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "pegboard.runner.{}", self.runner_id)
	}
}

pub struct GatewayReceiverSubject {
	gateway_id: Uuid,
}

impl GatewayReceiverSubject {
	pub fn new(gateway_id: Uuid) -> Self {
		Self { gateway_id }
	}
}

impl std::fmt::Display for GatewayReceiverSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "pegboard.gateway.{}", self.gateway_id)
	}
}
