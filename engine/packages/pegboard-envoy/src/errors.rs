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
		"invalid_request",
		"The websocket could not open due to an invalid request.",
		"Invalid websocket request: {0}."
	)]
	InvalidRequest(&'static str),
	#[error(
		"invalid_packet",
		"The websocket could not process the given packet.",
		"Invalid packet: {0}"
	)]
	InvalidPacket(String),
	#[error("invalid_url", "The connection URL is invalid.", "Invalid url: {0}")]
	InvalidUrl(String),
}
