mod common;

use regex::Regex;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingResponse, RoutingTimeout};
use rivet_guard_core::types::{EndpointType, GameGuardProtocol};
use rivet_guard_core::util::{build_actor_hostname_and_path, build_actor_hostname_and_path_regex};

#[test]
fn test_build_actor_hostname_and_path_regex() {
	// Test hostname-based routing
	let (hostname_regex, _) =
		build_actor_hostname_and_path_regex(EndpointType::Hostname, "example.com")
			.unwrap()
			.unwrap();

	// Test actor ID with port name in hostname
	assert!(hostname_regex.is_match("123e4567-e89b-12d3-a456-426614174000-http.example.com"));

	// Extract actor ID and port name
	let captures = hostname_regex
		.captures("123e4567-e89b-12d3-a456-426614174000-http.example.com")
		.unwrap();
	assert_eq!(
		captures.name("actor_id").unwrap().as_str(),
		"123e4567-e89b-12d3-a456-426614174000"
	);
	assert_eq!(captures.name("port_name").unwrap().as_str(), "http");

	// Test path-based routing
	let (_, path_regex) = build_actor_hostname_and_path_regex(EndpointType::Path, "example.com")
		.unwrap()
		.unwrap();

	// Path-based routing should have a path regex
	let path_regex = path_regex.unwrap();

	// Test actor ID with port name in path
	assert!(path_regex.is_match("/123e4567-e89b-12d3-a456-426614174000-http"));
	assert!(path_regex.is_match("/123e4567-e89b-12d3-a456-426614174000-http/additional/path"));

	// Extract actor ID and port name from path
	let captures = path_regex
		.captures("/123e4567-e89b-12d3-a456-426614174000-http/some/path")
		.unwrap();
	assert_eq!(
		captures.name("actor_id").unwrap().as_str(),
		"123e4567-e89b-12d3-a456-426614174000"
	);
	assert_eq!(captures.name("port_name").unwrap().as_str(), "http");
}

#[test]
fn test_build_actor_hostname_and_path() {
	let actor_id = "123e4567-e89b-12d3-a456-426614174000".parse().unwrap();
	let port_name = "http";
	let guard_hostname = "example.com";

	// Test hostname-based routing
	let hostname = build_actor_hostname_and_path(
		EndpointType::Hostname,
		&actor_id,
		port_name,
		guard_hostname,
		GameGuardProtocol::Http,
	)
	.unwrap();

	assert_eq!(
		hostname,
		"123e4567-e89b-12d3-a456-426614174000-http.example.com"
	);

	// Test path-based routing
	let path = build_actor_hostname_and_path(
		EndpointType::Path,
		&actor_id,
		port_name,
		guard_hostname,
		GameGuardProtocol::Http,
	)
	.unwrap();

	assert_eq!(path, "/123e4567-e89b-12d3-a456-426614174000-http");
}

#[test]
fn test_invalid_inputs() {
	// Test with empty guard hostname
	let result = build_actor_hostname_and_path_regex(EndpointType::Hostname, "");
	assert!(result.is_err() || result.unwrap().is_none());

	// Test with invalid UUID in hostname
	let hostname_regex = build_actor_hostname_and_path_regex(EndpointType::Hostname, "example.com")
		.unwrap()
		.unwrap()
		.0;

	// Should not match invalid UUID
	assert!(!hostname_regex.is_match("invalid-uuid-http.example.com"));
}
