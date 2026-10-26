//! Request classes give every kind of UPS request its own defined capacity.

use rivet_data::generated::ups_request_v1;

/// The kind of traffic a UPS request or stream carries. Every call site names one, and each class
/// has its own configured caps so one kind of traffic cannot starve another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RequestClass {
	/// Depot log shard requests: appends, commits, trims and replica probes.
	DepotLog,
	/// Depot volume lifecycle and filesystem requests.
	DepotVolume,
	/// Depot KV commands, including pegboard and Epoxy commands.
	DepotKvCommand,
	/// Depot tablet coordinator requests.
	DepotCoordinator,
	/// Depot stop-readiness checks during drain.
	DepotStopReadiness,
	/// Actor start and stop commands sent to an envoy connection.
	EnvoyCommand,
	/// Gateway HTTP tunnels to an envoy connection.
	GatewayHttp,
	/// Gateway WebSocket tunnels, held for the socket's life.
	GatewayWebSocket,
	/// Actor start notices from an actor's host to the gateway workers waiting for its start.
	ActorStartNotice,
}

/// Where a class's incoming cap comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassScope {
	/// Travels only on a reserved subject; the subject window is its incoming cap.
	Subject,
	/// Has its own process-wide incoming and outgoing caps.
	Process,
}

impl RequestClass {
	pub const ALL: [RequestClass; 9] = [
		RequestClass::DepotLog,
		RequestClass::DepotVolume,
		RequestClass::DepotKvCommand,
		RequestClass::DepotCoordinator,
		RequestClass::DepotStopReadiness,
		RequestClass::EnvoyCommand,
		RequestClass::GatewayHttp,
		RequestClass::GatewayWebSocket,
		RequestClass::ActorStartNotice,
	];

	/// The metric label and config key for this class.
	pub fn as_str(self) -> &'static str {
		match self {
			RequestClass::DepotLog => "depot_log",
			RequestClass::DepotVolume => "depot_volume",
			RequestClass::DepotKvCommand => "depot_kv_command",
			RequestClass::DepotCoordinator => "depot_coordinator",
			RequestClass::DepotStopReadiness => "depot_stop_readiness",
			RequestClass::EnvoyCommand => "envoy_command",
			RequestClass::GatewayHttp => "gateway_http",
			RequestClass::GatewayWebSocket => "gateway_websocket",
			RequestClass::ActorStartNotice => "actor_start_notice",
		}
	}

	pub fn scope(self) -> ClassScope {
		match self {
			RequestClass::DepotLog
			| RequestClass::DepotVolume
			| RequestClass::DepotKvCommand
			| RequestClass::DepotCoordinator
			| RequestClass::DepotStopReadiness => ClassScope::Subject,
			RequestClass::EnvoyCommand
			| RequestClass::GatewayHttp
			| RequestClass::GatewayWebSocket
			| RequestClass::ActorStartNotice => ClassScope::Process,
		}
	}
}

impl From<RequestClass> for ups_request_v1::RequestClass {
	fn from(class: RequestClass) -> Self {
		match class {
			RequestClass::DepotLog => ups_request_v1::RequestClass::DepotLog,
			RequestClass::DepotVolume => ups_request_v1::RequestClass::DepotVolume,
			RequestClass::DepotKvCommand => ups_request_v1::RequestClass::DepotKvCommand,
			RequestClass::DepotCoordinator => ups_request_v1::RequestClass::DepotCoordinator,
			RequestClass::DepotStopReadiness => ups_request_v1::RequestClass::DepotStopReadiness,
			RequestClass::EnvoyCommand => ups_request_v1::RequestClass::EnvoyCommand,
			RequestClass::GatewayHttp => ups_request_v1::RequestClass::GatewayHttp,
			RequestClass::GatewayWebSocket => ups_request_v1::RequestClass::GatewayWebSocket,
			RequestClass::ActorStartNotice => ups_request_v1::RequestClass::ActorStartNotice,
		}
	}
}

impl From<ups_request_v1::RequestClass> for RequestClass {
	fn from(class: ups_request_v1::RequestClass) -> Self {
		match class {
			ups_request_v1::RequestClass::DepotLog => RequestClass::DepotLog,
			ups_request_v1::RequestClass::DepotVolume => RequestClass::DepotVolume,
			ups_request_v1::RequestClass::DepotKvCommand => RequestClass::DepotKvCommand,
			ups_request_v1::RequestClass::DepotCoordinator => RequestClass::DepotCoordinator,
			ups_request_v1::RequestClass::DepotStopReadiness => RequestClass::DepotStopReadiness,
			ups_request_v1::RequestClass::EnvoyCommand => RequestClass::EnvoyCommand,
			ups_request_v1::RequestClass::GatewayHttp => RequestClass::GatewayHttp,
			ups_request_v1::RequestClass::GatewayWebSocket => RequestClass::GatewayWebSocket,
			ups_request_v1::RequestClass::ActorStartNotice => RequestClass::ActorStartNotice,
		}
	}
}
