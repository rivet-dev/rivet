use rivet_error::*;
use serde::Serialize;

#[derive(RivetError, Debug)]
#[error("ws")]
pub enum WsError {
	#[error(
		"eviction",
		"The websocket has been evicted and should not attempt to reconnect."
	)]
	Eviction,
	#[error(
		"going_away",
		"The Rivet Engine is migrating. The websocket should attempt to reconnect as soon as possible."
	)]
	GoingAway,
	#[error(
		"no_runner_config",
		"Must create a runner config before connecting an envoy with pool name {pool_name:?}."
	)]
	NoRunnerConfig { pool_name: String },
	#[error("timed_out", "Ping timed out.")]
	TimedOut,
	#[error(
		"registration_expired",
		"The envoy registration expired while its connection was still active. The websocket should reconnect."
	)]
	RegistrationExpired,
	#[error(
		"invalid_request",
		"The websocket could not open due to an invalid request.",
		"Invalid websocket request: {0}."
	)]
	InvalidRequest(&'static str),
}
