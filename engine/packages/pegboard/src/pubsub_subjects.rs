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

pub struct RunnerEvictionByIdSubject {
	runner_id: Id,
}

impl RunnerEvictionByIdSubject {
	pub fn new(runner_id: Id) -> Self {
		Self { runner_id }
	}
}

impl std::fmt::Display for RunnerEvictionByIdSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "pegboard.runner.eviction-by-id.{}", self.runner_id)
	}
}

pub struct RunnerEvictionByNameSubject {
	namespace_id: Id,
	runner_name: String,
	runner_key: String,
}

impl RunnerEvictionByNameSubject {
	pub fn new(namespace_id: Id, runner_name: &str, runner_key: &str) -> Self {
		Self {
			namespace_id,
			runner_name: runner_name.to_string(),
			runner_key: runner_key.to_string(),
		}
	}
}

impl std::fmt::Display for RunnerEvictionByNameSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"pegboard.runner.eviction-by-name.{}.{}.{}",
			self.namespace_id, self.runner_name, self.runner_key
		)
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
