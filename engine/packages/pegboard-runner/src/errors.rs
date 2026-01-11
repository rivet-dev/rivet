use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"response_body_too_large",
	"Response body too large.",
	"Response body size {size} bytes exceeds maximum allowed {max_size} bytes."
)]
pub struct ResponseBodyTooLarge {
	pub size: usize,
	pub max_size: usize,
}

#[derive(RivetError, Debug)]
#[error("ws")]
pub enum WsError {
	#[error("connection_closed", "Normal connection close.")]
	ConnectionClosed,
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
		"timed_out_waiting_for_init",
		"Timed out waiting for the init packet to be sent."
	)]
	TimedOutWaitingForInit,
	#[error(
		"invalid_initial_packet",
		"The websocket could not process the initial packet.",
		"Invalid initial packet: {0}."
	)]
	InvalidInitialPacket(&'static str),
	#[error(
		"invalid_packet",
		"The websocket could not process the given packet.",
		"Invalid packet: {0}"
	)]
	InvalidPacket(String),
	#[error("invalid_url", "The connection URL is invalid.", "Invalid url: {0}")]
	InvalidUrl(String),
}
