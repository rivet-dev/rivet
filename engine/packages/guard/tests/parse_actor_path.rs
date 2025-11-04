use rivet_guard::routing::parse_actor_path;

#[test]
fn test_parse_actor_path_with_token() {
	// Basic path with token using @ syntax
	let path = "/gateway/actor-123@my-token/api/v1/endpoint";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, Some("my-token".to_string()));
	assert_eq!(result.remaining_path, "/api/v1/endpoint");
}

#[test]
fn test_parse_actor_path_without_token() {
	// Path without token
	let path = "/gateway/actor-123/api/v1/endpoint";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/api/v1/endpoint");
}

#[test]
fn test_parse_actor_path_with_uuid() {
	// Path with UUID as actor ID
	let path = "/gateway/12345678-1234-1234-1234-123456789abc/status";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "12345678-1234-1234-1234-123456789abc");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/status");
}

#[test]
fn test_parse_actor_path_with_query_params() {
	// Path with query parameters
	let path = "/gateway/actor-456/api/endpoint?foo=bar&baz=qux";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-456");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/api/endpoint?foo=bar&baz=qux");

	// Path with token and query parameters
	let path = "/gateway/actor-456@token123/api?key=value";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-456");
	assert_eq!(result.token, Some("token123".to_string()));
	assert_eq!(result.remaining_path, "/api?key=value");
}

#[test]
fn test_parse_actor_path_with_fragment() {
	// Path with fragment
	let path = "/gateway/actor-789/page#section";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-789");
	assert_eq!(result.token, None);
	// Fragment is stripped during parsing
	assert_eq!(result.remaining_path, "/page");
}

#[test]
fn test_parse_actor_path_empty_remaining() {
	// Path with no remaining path
	let path = "/gateway/actor-000";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-000");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/");

	// With token and no remaining path
	let path = "/gateway/actor-000@tok";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-000");
	assert_eq!(result.token, Some("tok".to_string()));
	assert_eq!(result.remaining_path, "/");
}

#[test]
fn test_parse_actor_path_with_trailing_slash() {
	// Path with trailing slash
	let path = "/gateway/actor-111/api/";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-111");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/api/");
}

#[test]
fn test_parse_actor_path_complex_remaining() {
	// Complex remaining path with multiple segments
	let path = "/gateway/actor-complex@secure-token/api/v2/users/123/profile/settings";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-complex");
	assert_eq!(result.token, Some("secure-token".to_string()));
	assert_eq!(result.remaining_path, "/api/v2/users/123/profile/settings");
}

#[test]
fn test_parse_actor_path_special_characters() {
	// Actor ID with allowed special characters
	let path = "/gateway/actor_id-123.test/endpoint";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor_id-123.test");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/endpoint");
}

#[test]
fn test_parse_actor_path_encoded_characters() {
	// URL encoded characters in path
	let path = "/gateway/actor-123/api%20endpoint/test%2Fpath";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/api%20endpoint/test%2Fpath");
}

// Invalid path tests

#[test]
fn test_parse_actor_path_invalid_prefix() {
	// Wrong prefix
	assert!(parse_actor_path("/api/123/endpoint").is_none());
	assert!(parse_actor_path("/actor/123/endpoint").is_none());
}

#[test]
fn test_parse_actor_path_too_short() {
	// Too few segments
	assert!(parse_actor_path("/gateway").is_none());
}

#[test]
fn test_parse_actor_path_malformed_token() {
	// Token without actor_id (empty before @)
	assert!(parse_actor_path("/gateway/@tok/api").is_none());
	// Empty token (nothing after @)
	assert!(parse_actor_path("/gateway/actor-123@/api").is_none());
}

#[test]
fn test_parse_actor_path_empty_values() {
	// Empty actor_id
	assert!(parse_actor_path("/gateway//endpoint").is_none());
}

#[test]
fn test_parse_actor_path_double_slash() {
	// Double slashes in path
	let path = "/gateway//actor-123/endpoint";
	// This will fail because the double slash creates an empty segment
	assert!(parse_actor_path(path).is_none());
}

#[test]
fn test_parse_actor_path_case_sensitive() {
	// Keywords are case sensitive
	assert!(parse_actor_path("/Gateway/123/endpoint").is_none());
}

#[test]
fn test_parse_actor_path_query_and_fragment() {
	// Path with both query and fragment
	let path = "/gateway/actor-123/api?query=1#section";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, None);
	// Fragment is stripped, query is preserved
	assert_eq!(result.remaining_path, "/api?query=1");
}

#[test]
fn test_parse_actor_path_only_query_string() {
	// Path ending after actor_id but having query string
	let path = "/gateway/actor-123?direct=true";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, None);
	assert_eq!(result.remaining_path, "/?direct=true");
}

#[test]
fn test_parse_actor_path_token_with_special_chars() {
	// Token containing special characters
	let path = "/gateway/actor-123@token_with-chars.123/endpoint";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, Some("token_with-chars.123".to_string()));
	assert_eq!(result.remaining_path, "/endpoint");
}

#[test]
fn test_parse_actor_path_multiple_at_signs() {
	// Multiple @ signs - only first one is used for token splitting
	let path = "/gateway/actor-123@token@with@ats/endpoint";
	let result = parse_actor_path(path).unwrap();
	assert_eq!(result.actor_id, "actor-123");
	assert_eq!(result.token, Some("token@with@ats".to_string()));
	assert_eq!(result.remaining_path, "/endpoint");
}
