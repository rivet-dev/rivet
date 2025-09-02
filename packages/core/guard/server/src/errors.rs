use gas::prelude::Id;
use rivet_error::RivetError;
use serde::Serialize;

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"missing_header",
	"Missing header required for routing.",
	"Missing {header} header required for routing."
)]
pub struct MissingHeader {
	pub header: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"no_route",
	"No route found.",
	"No route found for hostname {host}, path {path}."
)]
pub struct NoRoute {
	pub host: String,
	pub path: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"wrong_addr_protocol",
	"Attempted to access a address using the wrong protocol.",
	"Attempted to access {expected} address \"{addr_name}\" using the wrong protocol: {received}"
)]
pub struct WrongAddrProtocol {
	pub addr_name: String,
	pub expected: &'static str,
	pub received: &'static str,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"actor_not_found",
	"Actor not found.",
	"Actor with ID {actor_id} and port {port_name} not found."
)]
pub struct ActorNotFound {
	pub actor_id: Id,
	pub port_name: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"actor_destroyed",
	"Actor destroyed.",
	"Actor {actor_id} was destroyed."
)]
pub struct ActorDestroyed {
	pub actor_id: Id,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"actor_ready_timeout",
	"Timed out waiting for actor to become ready. Ensure that the runner name selector is accurate and there are runners available in the namespace you created this actor."
)]
pub struct ActorReadyTimeout {
	pub actor_id: Id,
}
