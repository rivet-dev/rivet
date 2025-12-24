use gas::prelude::Id;
use rivet_error::RivetError;
use serde::Serialize;

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"missing_header",
	"Missing header required for routing.",
	"Missing {header} header."
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
	"actor_ready_timeout",
	"Timed out waiting for actor to become ready. Ensure that the runner name selector is accurate and there are runners available in the namespace you created this actor."
)]
pub struct ActorReadyTimeout {
	pub actor_id: Id,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"must_use_regional_host",
	"Request must use a regional URL for this datacenter.",
	"Invalid host {host} for datacenter {datacenter}. Please use one of the following hosts: {valid_hosts}"
)]
pub struct MustUseRegionalHost {
	pub host: String,
	pub datacenter: String,
	pub valid_hosts: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"actor_runner_failed",
	"Actor's runner pool is experiencing errors."
)]
pub struct ActorRunnerFailed {
	pub actor_id: Id,
}
