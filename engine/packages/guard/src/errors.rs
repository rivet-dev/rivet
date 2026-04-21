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
	"missing_query_parameter",
	"Missing query parameter required for routing.",
	"Missing {parameter} query parameter."
)]
pub struct MissingQueryParameter {
	pub parameter: String,
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
	"Timed out waiting for actor to become ready. Ensure that the pool selector is accurate and there are envoys available in the namespace you created this actor."
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

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"query_invalid_params",
	"invalid query gateway params",
	"invalid query gateway params: {detail}"
)]
pub struct QueryInvalidParams {
	pub detail: String,
}

#[derive(RivetError)]
#[error(
	"guard",
	"query_path_token_syntax",
	"query gateway paths must not use @token syntax"
)]
pub struct QueryPathTokenSyntax;

#[derive(RivetError)]
#[error(
	"guard",
	"query_get_disallowed_params",
	"query gateway method=get does not allow rvt-input, rvt-region, rvt-crash-policy, or rvt-pool params"
)]
pub struct QueryGetDisallowedParams;

#[derive(RivetError)]
#[error(
	"guard",
	"query_missing_pool",
	"query gateway method=getOrCreate requires rvt-pool param"
)]
pub struct QueryMissingPool;

#[derive(RivetError)]
#[error(
	"guard",
	"query_empty_actor_name",
	"query gateway actor name must not be empty"
)]
pub struct QueryEmptyActorName;

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"query_duplicate_param",
	"duplicate query gateway param",
	"duplicate query gateway param: {name}"
)]
pub struct QueryDuplicateParam {
	pub name: String,
}

#[derive(RivetError)]
#[error(
	"guard",
	"query_invalid_base64_input",
	"invalid base64url in query gateway input"
)]
pub struct QueryInvalidBase64Input;

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"query_invalid_cbor_input",
	"invalid query gateway input cbor",
	"invalid query gateway input cbor: {detail}"
)]
pub struct QueryInvalidCborInput {
	pub detail: String,
}

#[derive(RivetError, Serialize)]
#[error(
	"guard",
	"query_invalid_percent_encoding",
	"invalid percent-encoding for query gateway param",
	"invalid percent-encoding for query gateway param '{name}'"
)]
pub struct QueryInvalidPercentEncoding {
	pub name: String,
}
