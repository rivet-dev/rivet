// Keep this test suite in sync with the TypeScript equivalent at
// rivetkit-typescript/packages/rivetkit/tests/parse-actor-path.test.ts
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rivet_guard::routing::actor_path::{
	ParsedActorPath, QueryActorQuery, is_actor_gateway_path, parse_actor_path,
};

#[test]
fn parses_direct_actor_paths_with_existing_behavior() {
	let path = "/gateway/actor%2D123@tok%40en/api/v1/endpoint?foo=bar#section";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Direct(path) => {
			assert_eq!(path.actor_id, "actor-123");
			assert_eq!(path.token.as_deref(), Some("tok@en"));
			assert_eq!(path.stripped_path, "/api/v1/endpoint?foo=bar");
		}
		ParsedActorPath::Query(_) => panic!("expected direct actor path"),
	}
}

#[test]
fn parses_query_actor_get_paths() {
	let path = "/gateway/lobby/inspect?rvt-namespace=prod&rvt-method=get&rvt-key=region-west%2F1,shard-2,alpha%40beta&rvt-token=guard%2Ftoken&watch=1";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.token.as_deref(), Some("guard/token"));
			assert_eq!(path.stripped_path, "/inspect?watch=1");
			assert_eq!(
				path.query,
				QueryActorQuery::Get {
					namespace: "prod".to_string(),
					name: "lobby".to_string(),
					key: vec![
						"region-west/1".to_string(),
						"shard-2".to_string(),
						"alpha@beta".to_string(),
					],
					bypass_connectable: false,
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_query_actor_get_or_create_paths_with_input_and_region() {
	let input_bytes = vec![
		0xa2, 0x65, b'c', b'o', b'u', b'n', b't', 0x02, 0x67, b'e', b'n', b'a', b'b', b'l', b'e',
		b'd', 0xf5,
	];
	let input = encode_cbor_base64url(&input_bytes);
	let path = format!(
		"/gateway/worker/connect?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-key=shard-1&rvt-input={input}&rvt-region=us-west-2",
	);

	let result = parse_actor_path(&path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.token, None);
			assert_eq!(path.stripped_path, "/connect");
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "worker".to_string(),
					pool_name: "default".to_string(),
					key: vec!["shard-1".to_string()],
					input: Some(input_bytes),
					region: Some("us-west-2".to_string()),
					crash_policy: None,
					bypass_connectable: false,
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_query_actor_get_or_create_paths_with_multi_component_key() {
	let input_bytes = vec![0x65, b'h', b'e', b'l', b'l', b'o'];
	let input = encode_cbor_base64url(&input_bytes);
	let path = format!(
		"/gateway/worker/socket?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-key=tenant,job&rvt-input={input}"
	);

	let result = parse_actor_path(&path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "worker".to_string(),
					pool_name: "default".to_string(),
					key: vec!["tenant".to_string(), "job".to_string()],
					input: Some(input_bytes),
					region: None,
					crash_policy: None,
					bypass_connectable: false,
				}
			);
			assert_eq!(path.stripped_path, "/socket");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_query_actor_get_paths_with_empty_key() {
	let path = "/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-key=";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::Get {
					namespace: "default".to_string(),
					name: "lobby".to_string(),
					key: Vec::new(),
					bypass_connectable: false,
				}
			);
			assert_eq!(path.stripped_path, "/");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn omits_key_when_not_present() {
	let path = "/gateway/builder?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "builder".to_string(),
					pool_name: "default".to_string(),
					key: Vec::new(),
					input: None,
					region: None,
					crash_policy: None,
					bypass_connectable: false,
				}
			);
			assert_eq!(path.stripped_path, "/");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_simple_multi_component_keys() {
	let path = "/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-key=a,b,c";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::Get {
					namespace: "default".to_string(),
					name: "lobby".to_string(),
					key: vec!["a".to_string(), "b".to_string(), "c".to_string()],
					bypass_connectable: false,
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_crash_policy_param() {
	let path = "/gateway/worker?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-crash-policy=restart";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "worker".to_string(),
					pool_name: "default".to_string(),
					key: Vec::new(),
					input: None,
					region: None,
					crash_policy: Some(rivet_types::actors::CrashPolicy::Restart),
					bypass_connectable: false,
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_bypass_connectable_query_bool_strings() {
	let path = "/gateway/worker/request/bypass?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-bypass_connectable=true";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "worker".to_string(),
					pool_name: "default".to_string(),
					key: Vec::new(),
					input: None,
					region: None,
					crash_policy: None,
					bypass_connectable: true,
				}
			);
			assert_eq!(path.stripped_path, "/request/bypass");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn identifies_gateway_paths_without_parsing_query_params() {
	assert!(is_actor_gateway_path(
		"/gateway/worker/request/bypass?rvt-bypass_connectable=true"
	));
	assert!(is_actor_gateway_path("/gateway/actor-id"));
	assert!(!is_actor_gateway_path("/request/bypass"));
	assert!(!is_actor_gateway_path("/gateway//worker"));
}

#[test]
fn strips_rvt_params_from_remaining_path() {
	let path = "/gateway/lobby/api/v1?rvt-namespace=prod&rvt-method=get&foo=bar&baz=qux";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.stripped_path, "/api/v1?foo=bar&baz=qux");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn strips_all_rvt_params_leaving_empty_query_string() {
	let path = "/gateway/lobby/ws?rvt-namespace=prod&rvt-method=get";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.stripped_path, "/ws");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn preserves_percent_encoding_in_actor_query_params() {
	// Actor params should pass through byte-for-byte, preserving %20 encoding
	// and not re-encoding to + or any other form.
	let path = "/gateway/lobby/api?rvt-namespace=default&rvt-method=get&callback=https%3A%2F%2Fexample.com&name=hello%20world";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.stripped_path,
				"/api?callback=https%3A%2F%2Fexample.com&name=hello%20world"
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn preserves_plus_in_actor_query_params() {
	// Actor params should preserve + literally, not re-encode to %2B or decode to space.
	let path =
		"/gateway/lobby/api?rvt-namespace=default&rvt-method=get&search=hello+world&tag=c%2B%2B";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.stripped_path, "/api?search=hello+world&tag=c%2B%2B");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn handles_interleaved_rvt_and_actor_params() {
	// rvt-* and actor params may be interleaved; actor params preserve order and encoding.
	let path = "/gateway/lobby/ws?foo=1&rvt-namespace=default&bar=2&rvt-method=get&baz=3";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.stripped_path, "/ws?foo=1&bar=2&baz=3");
			assert_eq!(
				path.query,
				QueryActorQuery::Get {
					namespace: "default".to_string(),
					name: "lobby".to_string(),
					key: Vec::new(),
					bypass_connectable: false,
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn decodes_plus_as_space_in_rvt_values() {
	// rvt-* values should decode + as space (form-urlencoded), while actor
	// params preserve + literally.
	let path =
		"/gateway/lobby/api?rvt-namespace=my+ns&rvt-method=get&rvt-key=hello+world&q=search+term";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::Get {
					namespace: "my ns".to_string(),
					name: "lobby".to_string(),
					key: vec!["hello world".to_string()],
					bypass_connectable: false,
				}
			);
			// Actor param + is preserved literally.
			assert_eq!(path.stripped_path, "/api?q=search+term");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn preserves_uppercase_and_lowercase_percent_encoding() {
	// Percent-encoding case (%2f vs %2F) should be preserved in actor params.
	let path = "/gateway/lobby/api?rvt-namespace=default&rvt-method=get&lower=%2f&upper=%2F";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.stripped_path, "/api?lower=%2f&upper=%2F");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn rejects_missing_namespace() {
	let err = parse_actor_path("/gateway/lobby?rvt-method=get")
		.unwrap_err()
		.to_string();
	assert!(
		err.contains("namespace"),
		"expected namespace error, got: {err}"
	);
}

#[test]
fn rejects_missing_method() {
	let err = parse_actor_path("/gateway/lobby?rvt-namespace=default")
		.unwrap_err()
		.to_string();
	assert!(err.contains("method"), "expected method error, got: {err}");
}

#[test]
fn rejects_invalid_query_method() {
	let err = parse_actor_path("/gateway/lobby?rvt-namespace=default&rvt-method=create")
		.unwrap_err()
		.to_string();
	assert!(
		err.contains("unknown method"),
		"expected method error, got: {err}"
	);
}

#[test]
fn rejects_unknown_query_params() {
	let err =
		parse_actor_path("/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-unknown=value")
			.unwrap_err()
			.to_string();
	assert!(
		err.contains("unknown field"),
		"expected unknown field error, got: {err}"
	);
}

#[test]
fn rejects_duplicate_query_params() {
	let err = parse_actor_path(
		"/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-method=getOrCreate",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains("duplicate query gateway param: rvt-method"));
}

#[test]
fn rejects_empty_query_actor_name() {
	let err = parse_actor_path("/gateway/?rvt-namespace=default&rvt-method=get")
		.unwrap_err()
		.to_string();
	assert!(
		err.contains("query gateway actor name must not be empty"),
		"expected empty name error, got: {err}"
	);
}

#[test]
fn rejects_invalid_base64url_input() {
	let err = parse_actor_path(
		"/gateway/lobby?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-input=*",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains("invalid base64url in query gateway input"));
}

#[test]
fn rejects_invalid_cbor_input() {
	let invalid_input = URL_SAFE_NO_PAD.encode(b"foo");
	let err = parse_actor_path(&format!(
		"/gateway/lobby?rvt-namespace=default&rvt-method=getOrCreate&rvt-runner=default&rvt-input={invalid_input}"
	))
	.unwrap_err()
	.to_string();
	assert!(err.contains("invalid query gateway input cbor"));
}

#[test]
fn rejects_raw_at_token_syntax_in_query_paths() {
	let err = parse_actor_path("/gateway/lobby@token/connect?rvt-namespace=default&rvt-method=get")
		.unwrap_err()
		.to_string();
	assert!(err.contains("query gateway paths must not use @token syntax"));
}

#[test]
fn rejects_input_for_get_queries() {
	let input = encode_cbor_base64url(&[
		0xa1, 0x65, b'h', b'e', b'l', b'l', b'o', 0x65, b'w', b'o', b'r', b'l', b'd',
	]);
	let err = parse_actor_path(&format!(
		"/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-input={input}"
	))
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow rvt-input, rvt-region, rvt-crash-policy, or rvt-pool params"
	));
}

#[test]
fn rejects_region_for_get_queries() {
	let err = parse_actor_path(
		"/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-region=us-east-1",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow rvt-input, rvt-region, rvt-crash-policy, or rvt-pool params"
	));
}

#[test]
fn rejects_crash_policy_for_get_queries() {
	let err = parse_actor_path(
		"/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-crash-policy=restart",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow rvt-input, rvt-region, rvt-crash-policy, or rvt-pool params"
	));
}

#[test]
fn rejects_runner_for_get_queries() {
	let err =
		parse_actor_path("/gateway/lobby?rvt-namespace=default&rvt-method=get&rvt-runner=default")
			.unwrap_err()
			.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow rvt-input, rvt-region, rvt-crash-policy, or rvt-pool params"
	));
}

#[test]
fn rejects_missing_runner_for_get_or_create_queries() {
	let err = parse_actor_path("/gateway/lobby?rvt-namespace=default&rvt-method=getOrCreate")
		.unwrap_err()
		.to_string();
	assert!(err.contains("query gateway method=getOrCreate requires rvt-pool param"));
}

#[test]
fn strips_empty_parts_from_consecutive_ampersands() {
	// Malformed query strings with consecutive && should not produce empty parts
	// in the remaining query string.
	let path = "/gateway/lobby/api?rvt-namespace=default&&rvt-method=get&&foo=bar&&baz=qux";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(path.stripped_path, "/api?foo=bar&baz=qux");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn preserves_non_gateway_paths_as_none() {
	assert!(parse_actor_path("/actors/lobby").unwrap().is_none());
}

fn encode_cbor_base64url(bytes: &[u8]) -> String {
	URL_SAFE_NO_PAD.encode(bytes)
}
