use rivet_util::Id;

pub struct TunnelHttpRunnerSubject<'a> {
	runner_id: Id,
	port_name: &'a str,
}

impl<'a> TunnelHttpRunnerSubject<'a> {
	pub fn new(runner_id: Id, port_name: &'a str) -> Self {
		Self {
			runner_id,
			port_name,
		}
	}
}

impl std::fmt::Display for TunnelHttpRunnerSubject<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"pegboard.tunnel_http.runner.{}.{}",
			self.runner_id, self.port_name
		)
	}
}

pub struct TunnelHttpResponseSubject<'a> {
	runner_id: Id,
	port_name: &'a str,
	request_id: u64,
}

impl<'a> TunnelHttpResponseSubject<'a> {
	pub fn new(runner_id: Id, port_name: &'a str, request_id: u64) -> Self {
		Self {
			runner_id,
			port_name,
			request_id,
		}
	}
}

impl std::fmt::Display for TunnelHttpResponseSubject<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"pegboard.tunnel.http.request.{}.{}.{}",
			self.runner_id, self.port_name, self.request_id
		)
	}
}

pub struct TunnelHttpWebSocketSubject<'a> {
	runner_id: Id,
	port_name: &'a str,
	websocket_id: u64,
}

impl<'a> TunnelHttpWebSocketSubject<'a> {
	pub fn new(runner_id: Id, port_name: &'a str, websocket_id: u64) -> Self {
		Self {
			runner_id,
			port_name,
			websocket_id,
		}
	}
}

impl std::fmt::Display for TunnelHttpWebSocketSubject<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"pegboard.tunnel.http.websocket.{}.{}.{}",
			self.runner_id, self.port_name, self.websocket_id
		)
	}
}
