use global_error::prelude::*;
use uuid::Uuid;

use crate::types::{EndpointType, GameGuardProtocol};

/// Build a hostname or path string for the given endpoint type
pub fn build_actor_hostname_and_path(
	endpoint_type: EndpointType,
	actor_id: &Uuid,
	port_name: &str,
	guard_hostname: &str,
	_protocol: GameGuardProtocol,
) -> GlobalResult<String> {
	match endpoint_type {
		EndpointType::Hostname => {
			// For hostname, we create: {actor_id}-{port_name}.{guard_hostname}
			Ok(format!("{}-{}.{}", actor_id, port_name, guard_hostname))
		}
		EndpointType::Path => {
			// For path, we create: /{actor_id}-{port_name}
			Ok(format!("/{}-{}", actor_id, port_name))
		}
	}
}
