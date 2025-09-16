use gas::prelude::*;

pub struct TunnelRunnerReceiverSubject<'a> {
	runner_key: &'a str,
	port_name: &'a str,
}

impl<'a> TunnelRunnerReceiverSubject<'a> {
	pub fn new(runner_key: &'a str, port_name: &'a str) -> Self {
		Self {
			runner_key,
			port_name,
		}
	}
}

impl std::fmt::Display for TunnelRunnerReceiverSubject<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"pegboard.tunnel.runner_receiver.{}.{}",
			self.runner_key, self.port_name
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
