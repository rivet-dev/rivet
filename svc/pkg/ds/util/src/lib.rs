pub mod consts;
pub mod test;

use rivet_operation::prelude::*;

pub fn build_ds_hostname(
	server_id: Uuid,
	port_name: &str,
	datacenter_id: Uuid,
) -> GlobalResult<String> {
	// TODO: Change lobby -> server
	Ok(format!(
		"{}-{}.lobby.{}.{}",
		server_id,
		port_name,
		datacenter_id,
		unwrap!(rivet_util::env::domain_job()),
	))
}

/// Formats the port label to be used in Nomad.
///
/// Prefixing this port ensure that the user defined port names don't interfere
/// with other ports.
///
/// See also SQL `concat` in `svc/api/traefik-provider/src/route/game_guard/dynamic_servers.rs`.
pub fn format_nomad_port_label(port_label: &str) -> String {
	let snake_port_label = heck::SnakeCase::to_snake_case(port_label);
	format!("ds_{snake_port_label}")
}

pub const RUNC_SETUP_CPU: i32 = 50;
pub const RUNC_SETUP_MEMORY: i32 = 32;
pub const RUNC_CLEANUP_CPU: i32 = 50;
pub const RUNC_CLEANUP_MEMORY: i32 = 32;
