// Keep this test suite in sync with the TypeScript equivalent at
// rivetkit-typescript/packages/rivetkit/tests/parse-actor-path.test.ts
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rivet_guard::routing::actor_path::{ParsedActorPath, QueryActorQuery, parse_actor_path};

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
	let path = "/gateway/lobby;namespace=prod;method=get;key=region%2Cwest%2F1,,alpha%40beta;token=guard%2Ftoken/inspect?watch=1";
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
						"region,west/1".to_string(),
						"".to_string(),
						"alpha@beta".to_string(),
					],
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_query_actor_get_or_create_paths_with_input_and_region() {
	let input_bytes = vec![
		0xa2, 0x65, b'c', b'o', b'u', b'n', b't', 0x02, 0x67, b'e', b'n', b'a', b'b', b'l',
		b'e', b'd', 0xf5,
	];
	let input = encode_cbor_base64url(&input_bytes);
	let path = format!(
		"/gateway/worker;namespace=default;method=getOrCreate;runnerName=default;key=shard-1;input={input};region=us-west-2/connect"
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
					runner_name: "default".to_string(),
					key: vec!["shard-1".to_string()],
					input: Some(input_bytes),
					region: Some("us-west-2".to_string()),
					crash_policy: None,
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_query_actor_get_or_create_paths_with_empty_key_component() {
	let input_bytes = vec![0x65, b'h', b'e', b'l', b'l', b'o'];
	let input = encode_cbor_base64url(&input_bytes);
	let path = format!(
		"/gateway/worker;namespace=default;method=getOrCreate;runnerName=default;key=,;input={input}/socket"
	);

	let result = parse_actor_path(&path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "worker".to_string(),
					runner_name: "default".to_string(),
					key: vec!["".to_string(), "".to_string()],
					input: Some(input_bytes),
					region: None,
					crash_policy: None,
				}
			);
			assert_eq!(path.stripped_path, "/socket");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_query_actor_get_paths_with_key_equals_as_single_empty_component() {
	let path = "/gateway/lobby;namespace=default;method=get;key=";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::Get {
					namespace: "default".to_string(),
					name: "lobby".to_string(),
					key: vec!["".to_string()],
				}
			);
			assert_eq!(path.stripped_path, "/");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn omits_key_when_not_present() {
	let path = "/gateway/builder;namespace=default;method=getOrCreate;runnerName=default";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "builder".to_string(),
					runner_name: "default".to_string(),
					key: Vec::new(),
					input: None,
					region: None,
					crash_policy: None,
				}
			);
			assert_eq!(path.stripped_path, "/");
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn parses_crash_policy_param() {
	let path = "/gateway/worker;namespace=default;method=getOrCreate;runnerName=default;crashPolicy=restart";
	let result = parse_actor_path(path).unwrap().unwrap();

	match result {
		ParsedActorPath::Query(path) => {
			assert_eq!(
				path.query,
				QueryActorQuery::GetOrCreate {
					namespace: "default".to_string(),
					name: "worker".to_string(),
					runner_name: "default".to_string(),
					key: Vec::new(),
					input: None,
					region: None,
					crash_policy: Some(rivet_types::actors::CrashPolicy::Restart),
				}
			);
		}
		ParsedActorPath::Direct(_) => panic!("expected query actor path"),
	}
}

#[test]
fn rejects_missing_namespace() {
	let err = parse_actor_path("/gateway/lobby;method=get")
		.unwrap_err()
		.to_string();
	assert!(err.contains("namespace"), "expected namespace error, got: {err}");
}

#[test]
fn rejects_create_query_method() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=create")
		.unwrap_err()
		.to_string();
	assert!(err.contains("create"), "expected create error, got: {err}");
}

#[test]
fn rejects_unknown_query_params() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get;unknown=value")
		.unwrap_err()
		.to_string();
	assert!(err.contains("unknown query gateway param: unknown"));
}

#[test]
fn rejects_duplicate_query_params() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get;method=create")
		.unwrap_err()
		.to_string();
	assert!(err.contains("duplicate query gateway param: method"));
}

#[test]
fn rejects_name_as_matrix_param() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get;name=other")
		.unwrap_err()
		.to_string();
	assert!(err.contains("duplicate query gateway param: name"));
}

#[test]
fn rejects_namespace_as_matrix_param() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get;namespace=other")
		.unwrap_err()
		.to_string();
	assert!(err.contains("duplicate query gateway param: namespace"));
}

#[test]
fn rejects_query_params_missing_equals() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get;key")
		.unwrap_err()
		.to_string();
	assert!(err.contains("query gateway param is missing '=': key"));
}

#[test]
fn rejects_invalid_percent_encoding_in_name() {
	let err = parse_actor_path("/gateway/lobb%ZZy;namespace=default;method=get")
		.unwrap_err()
		.to_string();
	assert!(err.contains("invalid percent-encoding for query gateway param 'name'"));
}

#[test]
fn rejects_empty_query_actor_name() {
	let err = parse_actor_path("/gateway/;namespace=default;method=get")
		.unwrap_err()
		.to_string();
	assert!(err.contains("query gateway actor name must not be empty"));
}

#[test]
fn rejects_invalid_base64url_input() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=getOrCreate;runnerName=default;input=*")
		.unwrap_err()
		.to_string();
	assert!(err.contains("invalid base64url in query gateway input"));
}

#[test]
fn rejects_invalid_cbor_input() {
	let invalid_input = URL_SAFE_NO_PAD.encode(b"foo");
	let err = parse_actor_path(&format!(
		"/gateway/lobby;namespace=default;method=getOrCreate;runnerName=default;input={invalid_input}"
	))
	.unwrap_err()
	.to_string();
	assert!(err.contains("invalid query gateway input cbor"));
}

#[test]
fn rejects_raw_at_token_syntax_in_query_paths() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get@token/connect")
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
		"/gateway/lobby;namespace=default;method=get;input={input}"
	))
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow input, region, crashPolicy, or runnerName params"
	));
}

#[test]
fn rejects_region_for_get_queries() {
	let err = parse_actor_path("/gateway/lobby;namespace=default;method=get;region=us-east-1")
		.unwrap_err()
		.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow input, region, crashPolicy, or runnerName params"
	));
}

#[test]
fn rejects_crash_policy_for_get_queries() {
	let err = parse_actor_path(
		"/gateway/lobby;namespace=default;method=get;crashPolicy=restart",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow input, region, crashPolicy, or runnerName params"
	));
}

#[test]
fn rejects_runner_name_for_get_queries() {
	let err = parse_actor_path(
		"/gateway/lobby;namespace=default;method=get;runnerName=default",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=get does not allow input, region, crashPolicy, or runnerName params"
	));
}

#[test]
fn rejects_missing_runner_name_for_get_or_create_queries() {
	let err = parse_actor_path(
		"/gateway/lobby;namespace=default;method=getOrCreate",
	)
	.unwrap_err()
	.to_string();
	assert!(err.contains(
		"query gateway method=getOrCreate requires runnerName param"
	));
}

#[test]
fn preserves_non_gateway_paths_as_none() {
	assert!(parse_actor_path("/actors/lobby").unwrap().is_none());
}

fn encode_cbor_base64url(bytes: &[u8]) -> String {
	URL_SAFE_NO_PAD.encode(bytes)
}
