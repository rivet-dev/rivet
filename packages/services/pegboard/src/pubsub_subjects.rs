use gas::prelude::*;

pub struct TunnelRunnerReceiverSubject<'a> {
	namespace_id: Id,
	runner_name: &'a str,
	runner_key: &'a str,
}

impl<'a> TunnelRunnerReceiverSubject<'a> {
	pub fn new(namespace_id: Id, runner_name: &'a str, runner_key: &'a str) -> Self {
		Self {
			namespace_id,
			runner_name,
			runner_key,
		}
	}
}

impl std::fmt::Display for TunnelRunnerReceiverSubject<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"pegboard.tunnel.runner_receiver.{}.{}.{}",
			self.namespace_id, self.runner_name, self.runner_key
		)
	}
}

pub struct TunnelGatewayReceiverSubject {
	gateway_id: Uuid,
}

impl<'a> TunnelGatewayReceiverSubject {
	pub fn new(gateway_id: Uuid) -> Self {
		Self { gateway_id }
	}
}

impl std::fmt::Display for TunnelGatewayReceiverSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "pegboard.gateway.receiver.{}", self.gateway_id)
	}
}
