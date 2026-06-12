use std::borrow::Cow;

use gas::prelude::*;
use rivet_runner_protocol as protocol;
use universalpubsub::Subject;

#[derive(Clone)]
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

impl Subject for RunnerReceiverSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.runner"))
	}
}

#[derive(Clone)]
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

impl Subject for RunnerEvictionByIdSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.runner.eviction-by-id"))
	}
}

#[derive(Clone)]
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

impl Subject for RunnerEvictionByNameSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.runner.eviction-by-name"))
	}
}

#[derive(Clone)]
pub struct GatewayReceiverSubject {
	gateway_id: protocol::GatewayId,
}

impl GatewayReceiverSubject {
	pub fn new(gateway_id: protocol::GatewayId) -> Self {
		Self { gateway_id }
	}
}

impl std::fmt::Display for GatewayReceiverSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"pegboard.gateway.{}",
			protocol::util::id_to_string(&self.gateway_id)
		)
	}
}

impl Subject for GatewayReceiverSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.gateway"))
	}
}

#[derive(Clone)]
pub struct EnvoyReceiverSubject {
	namespace_id: Id,
	envoy_key: String,
}

impl EnvoyReceiverSubject {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		Self {
			namespace_id,
			envoy_key,
		}
	}
}

impl std::fmt::Display for EnvoyReceiverSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "pegboard.envoy.{}.{}", self.namespace_id, self.envoy_key)
	}
}

impl Subject for EnvoyReceiverSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.envoy"))
	}
}

#[derive(Clone)]
pub struct EnvoyEvictionSubject {
	namespace_id: Id,
	envoy_key: String,
}

impl EnvoyEvictionSubject {
	pub fn new(namespace_id: Id, envoy_key: String) -> Self {
		Self {
			namespace_id,
			envoy_key,
		}
	}
}

impl std::fmt::Display for EnvoyEvictionSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"pegboard.envoy.eviction.{}.{}",
			self.namespace_id, self.envoy_key
		)
	}
}

impl Subject for EnvoyEvictionSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.envoy.eviction"))
	}
}

#[derive(Clone)]
pub struct ServerlessOutboundSubject;

impl std::fmt::Display for ServerlessOutboundSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "pegboard.serverless.outbound",)
	}
}

impl Subject for ServerlessOutboundSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed("pegboard.serverless.outbound"))
	}
}
