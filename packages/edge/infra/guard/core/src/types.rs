use serde::{Deserialize, Serialize};

/// Type of endpoint to route to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointType {
	/// Route based on hostname
	Hostname,
	/// Route based on path
	Path,
}

/// Protocol for the game guard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameGuardProtocol {
	/// HTTP protocol
	Http,
	/// WebSocket protocol
	WebSocket,
}
