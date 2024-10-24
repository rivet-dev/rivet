use chirp_worker::prelude::*;
use regex::Regex;

use super::oci_config;
use crate::types::GameGuardProtocol;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProtocol {
	Tcp,
	Udp,
}

impl From<GameGuardProtocol> for TransportProtocol {
	fn from(proxy_protocol: GameGuardProtocol) -> Self {
		match proxy_protocol {
			GameGuardProtocol::Http
			| GameGuardProtocol::Https
			| GameGuardProtocol::Tcp
			| GameGuardProtocol::TcpTls => Self::Tcp,
			GameGuardProtocol::Udp => Self::Udp,
		}
	}
}

impl TransportProtocol {
	pub fn as_cni_protocol(&self) -> &'static str {
		match self {
			Self::Tcp => "tcp",
			Self::Udp => "udp",
		}
	}
}

/// Helper structure for parsing all of the runtime's ports before building the
/// config.
#[derive(Clone)]
pub struct DecodedPort {
	pub label: String,
	pub nomad_port_label: String,
	pub target: u16,
	pub proxy_protocol: GameGuardProtocol,
}

/// Build base config used to generate the OCI bundle's config.json.
pub fn gen_oci_bundle_config(
	cpu: u64,
	memory: u64,
	memory_max: u64,
	env: Vec<String>,
) -> GlobalResult<String> {
	let config_str = serde_json::to_string(&oci_config::config(cpu, memory, memory_max, env))?;

	// Escape Go template syntax
	let config_str = inject_consul_env_template(&config_str)?;

	Ok(config_str)
}

/// Makes user-generated string safe to inject in to a Go template.
pub fn escape_go_template(input: &str) -> String {
	let re = Regex::new(r"(\{\{|\}\})").unwrap();
	re.replace_all(input, r#"{{"$1"}}"#)
		.to_string()
		// TODO: This removes exploits to inject env vars (see below)
		// SVC-3307
		.replace("###", "")
}

/// Generates a template string that we can substitute with the real environment variable
///
/// This must be safe to inject in to a JSON string so it can be substituted after rendering the
/// JSON object. Intended to be used from within JSON.
///
/// See inject_consul_env_template.
pub fn template_env_var(name: &str) -> String {
	format!("###ENV:{name}###")
}

/// Like template_env_var, but removes surrounding quotes.
pub fn template_env_var_int(name: &str) -> String {
	format!("###ENV_INT:{name}###")
}

/// Substitutes env vars generated from template_env_var with Consul template syntax.
///
/// Intended to be used from within JSON.
pub fn inject_consul_env_template(input: &str) -> GlobalResult<String> {
	// Regular strings
	let re = Regex::new(r"###ENV:(\w+)###")?;
	let output = re
		.replace_all(input, r#"{{ env "$1" | regexReplaceAll "\"" "\\\"" }}"#)
		.to_string();

	// Integers
	let re = Regex::new(r####""###ENV_INT:(\w+)###""####)?;
	let output = re
		.replace_all(&output, r#"{{ env "$1" | regexReplaceAll "\"" "\\\"" }}"#)
		.to_string();

	Ok(output)
}

pub fn nomad_host_port_env_var(port_label: &str) -> String {
	format!("NOMAD_HOST_PORT_{}", port_label.replace('-', "_"))
}
