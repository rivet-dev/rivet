use global_error::prelude::*;
use regex::Regex;
use uuid::Uuid;

use crate::types::{EndpointType, GameGuardProtocol};

/// Build a hostname and path regex for the given endpoint type
///
/// Returns a tuple of (hostname_regex, path_regex) where path_regex is None for Hostname type
pub fn build_actor_hostname_and_path_regex(
    endpoint_type: EndpointType,
    guard_hostname: &str,
) -> GlobalResult<Option<(Regex, Option<Regex>)>> {
    if guard_hostname.is_empty() {
        return Ok(None);
    }

    match endpoint_type {
        EndpointType::Hostname => {
            // Format: {actor_id}-{port_name}.{guard_hostname}
            let hostname_pattern = format!(
                r"^(?P<actor_id>[0-9a-f]{{8}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{12}})-(?P<port_name>[a-zA-Z0-9_-]+)\.{}$",
                regex::escape(guard_hostname)
            );
            let hostname_regex = Regex::new(&hostname_pattern)?;
            Ok(Some((hostname_regex, None)))
        }
        EndpointType::Path => {
            // Hostname is just the guard hostname
            let hostname_pattern = format!("^{}$", regex::escape(guard_hostname));
            let hostname_regex = Regex::new(&hostname_pattern)?;

            // Path format: /{actor_id}-{port_name}(/.*)?
            let path_pattern = r"^/(?P<actor_id>[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})-(?P<port_name>[a-zA-Z0-9_-]+)(/.*)?$";
            let path_regex = Regex::new(path_pattern)?;

            Ok(Some((hostname_regex, Some(path_regex))))
        }
    }
}

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