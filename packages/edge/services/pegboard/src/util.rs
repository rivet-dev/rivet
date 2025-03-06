use chirp_workflow::prelude::*;
use cluster::types::GuardPublicHostname;

use crate::types::{EndpointType, GameGuardProtocol};

pub fn build_actor_hostname_and_path(
	server_id: Uuid,
	port_name: &str,
	protocol: GameGuardProtocol,
	endpoint_type: EndpointType,
	guard_public_hostname: &GuardPublicHostname,
) -> GlobalResult<(String, Option<String>)> {
	let is_http = matches!(protocol, GameGuardProtocol::Http | GameGuardProtocol::Https);
	match (is_http, endpoint_type, guard_public_hostname) {
		// Non-HTTP protocols can use any hostname (since they route by port), but including the
		// server in the subdomain is a convenience
		(true, EndpointType::Hostname, GuardPublicHostname::DnsParent(dns_parent))
		| (false, _, GuardPublicHostname::DnsParent(dns_parent)) => {
			Ok((format!("{server_id}-{port_name}.actor.{dns_parent}"), None))
		}

		(true, EndpointType::Hostname, GuardPublicHostname::Static(_)) => {
			bail!("cannot use hostname endpoint type with static hostname")
		}

		(true, EndpointType::Path, GuardPublicHostname::DnsParent(dns_parent)) => Ok((
			// This will not collide with host-based routing since server IDs are always UUIDs.
			//
			// This is stored on a subdomain of `actor` instead of `actor.{dns_parent}` since
			// hosting actors on a parent domain of the `{actor_id}.actor.{dns_parent}` could lead
			// to a security vulnerability if cookies on the parent domain have a misconfigured
			// domain scope that grants access to the children. This is a very niche security
			// vulnerability, but worth avoiding regardless.
			format!("route.actor.{dns_parent}"),
			Some(format!("/{server_id}-{port_name}")),
		)),

		(true, EndpointType::Path, GuardPublicHostname::Static(static_)) => {
			Ok((static_.clone(), Some(format!("/{server_id}-{port_name}"))))
		}

		// Non-HTTP protocols will be routed via the port, so we can use the static protocol
		(false, _, GuardPublicHostname::Static(static_)) => Ok((static_.clone(), None)),
	}
}

pub fn image_artifact_url_stub(
	config: &rivet_config::Config,
	upload_id: Uuid,
	file_name: &str,
) -> GlobalResult<String> {
	Ok(format!(
		"/s3-cache/{namespace}-bucket-build/{upload_id}/{file_name}",
		namespace = config.server()?.rivet.namespace,
	))
}

/// Standardize the port name format.
pub fn pegboard_normalize_port_name(port_name: &str) -> String {
	heck::SnakeCase::to_snake_case(port_name)
}
