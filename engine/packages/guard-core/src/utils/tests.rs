use hyper::header::HeaderValue;

use super::*;

#[test]
fn redact_uri_for_logs_replaces_token_query_values() {
	let uri: hyper::Uri = "/gateway/threadActor/request/retire?rvt-namespace=default&rvt-token=sk_secret&rvt-key=T-1&Token=abc&x=1"
		.parse()
		.expect("parse uri");

	assert_eq!(
		redact_uri_for_logs(&uri),
		"/gateway/threadActor/request/retire?rvt-namespace=default&rvt-token=REDACTED&rvt-key=T-1&Token=REDACTED&x=1",
	);
}

#[test]
fn redact_uri_for_logs_leaves_uris_without_credentials_unchanged() {
	let without_query: hyper::Uri = "/gateway/actor/action/run".parse().expect("parse uri");
	assert_eq!(
		redact_uri_for_logs(&without_query),
		"/gateway/actor/action/run"
	);

	let with_query: hyper::Uri = "/envoys/connect?namespace=default&name=k8s%20pool"
		.parse()
		.expect("parse uri");
	assert_eq!(
		redact_uri_for_logs(&with_query),
		"/envoys/connect?namespace=default&name=k8s%20pool",
	);
}

#[test]
fn redact_path_for_logs_replaces_token_query_values() {
	assert_eq!(
		redact_path_for_logs("/gateway/actor/action/run?rvt-namespace=default&rvt-token=sk_secret"),
		"/gateway/actor/action/run?rvt-namespace=default&rvt-token=REDACTED",
	);
	assert_eq!(redact_path_for_logs("/health"), "/health");
}

#[test]
fn redact_uri_for_logs_matches_percent_encoded_token_keys() {
	let uri: hyper::Uri = "http://example.com/gateway/a?rvt%2Dtoken=sk_secret"
		.parse()
		.expect("parse uri");

	assert_eq!(
		redact_uri_for_logs(&uri),
		"http://example.com/gateway/a?rvt%2Dtoken=REDACTED",
	);
}

#[test]
fn client_state_without_limits_admits_repeated_requests() {
	let mut state = ClientState::new(None, None);

	for _ in 0..10_001 {
		assert_eq!(state.try_admit(), Ok(()));
	}
}

#[test]
fn client_state_reports_rate_limit_separately() {
	let mut state = ClientState::new(Some((1, Duration::from_secs(60))), None);

	assert_eq!(state.try_admit(), Ok(()));
	assert_eq!(state.try_admit(), Err(AdmissionRejection::RateLimit));
}

#[test]
fn client_state_reports_max_in_flight_separately() {
	let mut state = ClientState::new(None, Some(1));

	assert_eq!(state.try_admit(), Ok(()));
	assert_eq!(state.try_admit(), Err(AdmissionRejection::MaxInFlight));
	state.release_in_flight();
	assert_eq!(state.try_admit(), Ok(()));
}

#[test]
fn labels_rivet_errors_by_group_and_code() {
	let err = crate::errors::UriParseError("http://actor-9f3b.example/path".to_owned()).build();

	assert_eq!("guard.uri_parse_error", error_metric_label(&err));
}

#[test]
fn labels_rivet_errors_wrapped_in_context() {
	let err = anyhow::Error::from(crate::errors::WebSocketNotSupported.build())
		.context("failed handling websocket for actor 9f3b");

	assert_eq!("guard.websocket_not_supported", error_metric_label(&err));
}

#[test]
fn labels_foreign_errors_by_type_name() {
	let err = anyhow::Error::from(std::io::Error::new(
		std::io::ErrorKind::ConnectionReset,
		"connection reset by peer from 10.0.0.4:52190",
	));

	assert_eq!("std::io::Error", error_metric_label(&err));
}

#[test]
fn labels_unknown_errors_without_leaking_the_message() {
	let err = anyhow::anyhow!("actor 9f3b on host example.com failed");

	assert_eq!(UNKNOWN_ERROR_LABEL, error_metric_label(&err));
}

#[test]
fn retries_guard_actor_ready_timeout_response() {
	let mut headers = hyper::HeaderMap::new();
	headers.insert(
		X_RIVET_ERROR,
		HeaderValue::from_static("guard.actor_ready_timeout"),
	);

	assert!(should_retry_request_inner(
		StatusCode::SERVICE_UNAVAILABLE,
		&headers,
	));
}

#[test]
fn skips_service_unavailable_without_rivet_error_header() {
	let headers = hyper::HeaderMap::new();

	assert!(!should_retry_request_inner(
		StatusCode::SERVICE_UNAVAILABLE,
		&headers,
	));
}

#[test]
fn skips_non_service_unavailable_with_rivet_error_header() {
	let mut headers = hyper::HeaderMap::new();
	headers.insert(X_RIVET_ERROR, HeaderValue::from_static("guard.no_route"));

	assert!(!should_retry_request_inner(StatusCode::NOT_FOUND, &headers));
}

#[test]
fn does_not_retry_unconfirmed_request_delivery() {
	let error = crate::errors::RequestDeliveryUnconfirmed {
		phase: "request_start".to_owned(),
		reason: "envoy_handoff_ack_timeout".to_owned(),
	}
	.build();

	assert!(!should_retry_error(&error));
}

#[test]
fn retries_a_definitive_no_responders_request_start_failure() {
	let error = crate::errors::TunnelMessageTimeout {
		phase: "request_start".to_owned(),
		reason: "no_responders_after_retry_budget_exhausted".to_owned(),
	}
	.build();

	assert!(should_retry_error(&error));
}

#[test]
fn structured_error_responses_include_the_matching_error_header() {
	let response = err_into_response(
		crate::errors::RequestDeliveryUnconfirmed {
			phase: "request_start".to_owned(),
			reason: "envoy_handoff_ack_timeout".to_owned(),
		}
		.build(),
	)
	.expect("build error response");

	assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
	assert_eq!(
		response.headers().get(X_RIVET_ERROR),
		Some(&HeaderValue::from_static(
			"guard.request_delivery_unconfirmed"
		)),
	);
}

fn test_request_context(remote_addr: &str, headers: hyper::HeaderMap) -> RequestContext {
	RequestContext::new(
		remote_addr.parse().expect("parse remote addr"),
		Id::nil(),
		Id::nil(),
		"example.com".to_string(),
		"/".to_string(),
		hyper::Method::GET,
		headers,
		false,
		remote_addr
			.rsplit_once(':')
			.expect("split port")
			.0
			.parse()
			.expect("parse client ip"),
		std::time::Instant::now(),
		tokio_util::sync::CancellationToken::new(),
		&rivet_config::config::guard::Guard::default(),
	)
}

#[test]
fn add_proxy_headers_preserves_repeated_header_values() {
	let mut headers = hyper::HeaderMap::new();
	headers.append(hyper::header::COOKIE, HeaderValue::from_static("a=1"));
	headers.append(hyper::header::COOKIE, HeaderValue::from_static("b=2"));
	let req_ctx = test_request_context("10.0.0.1:1234", headers);

	let mut proxied = hyper::HeaderMap::new();
	add_proxy_headers_with_addr(&mut proxied, &req_ctx).expect("add proxy headers");

	let cookies = proxied
		.get_all(hyper::header::COOKIE)
		.into_iter()
		.collect::<Vec<_>>();
	assert_eq!(cookies, vec!["a=1", "b=2"]);
}

#[test]
fn add_proxy_headers_keeps_the_generated_websocket_handshake() {
	let mut headers = hyper::HeaderMap::new();
	headers.insert(
		hyper::header::SEC_WEBSOCKET_KEY,
		HeaderValue::from_static("Zm9vYmFyYmF6cXV4MTIzNA=="),
	);
	headers.insert(
		hyper::header::SEC_WEBSOCKET_VERSION,
		HeaderValue::from_static("13"),
	);
	headers.insert(
		hyper::header::SEC_WEBSOCKET_PROTOCOL,
		HeaderValue::from_static("rivet"),
	);
	let req_ctx = test_request_context("10.0.0.1:1234", headers);

	// The outgoing websocket request generates its own handshake before the client's headers are
	// copied over, and verifies the upstream response against the key it generated.
	let mut proxied = hyper::HeaderMap::new();
	proxied.insert(
		hyper::header::SEC_WEBSOCKET_KEY,
		HeaderValue::from_static("Z2VuZXJhdGVka2V5MTIzNA=="),
	);
	proxied.insert(
		hyper::header::SEC_WEBSOCKET_VERSION,
		HeaderValue::from_static("13"),
	);
	add_proxy_headers_with_addr(&mut proxied, &req_ctx).expect("add proxy headers");

	let keys = proxied
		.get_all(hyper::header::SEC_WEBSOCKET_KEY)
		.into_iter()
		.collect::<Vec<_>>();
	assert_eq!(keys, vec!["Z2VuZXJhdGVka2V5MTIzNA=="]);

	let versions = proxied
		.get_all(hyper::header::SEC_WEBSOCKET_VERSION)
		.into_iter()
		.collect::<Vec<_>>();
	assert_eq!(versions, vec!["13"]);

	// The negotiated subprotocol is not part of the handshake guard generates, so it is forwarded.
	assert_eq!(
		proxied
			.get(hyper::header::SEC_WEBSOCKET_PROTOCOL)
			.expect("subprotocol"),
		"rivet"
	);
}

#[test]
fn add_proxy_headers_drops_hop_by_hop_headers() {
	let mut headers = hyper::HeaderMap::new();
	headers.insert(
		hyper::header::CONNECTION,
		HeaderValue::from_static("keep-alive"),
	);
	headers.insert(
		hyper::header::TRANSFER_ENCODING,
		HeaderValue::from_static("chunked"),
	);
	headers.insert(
		hyper::header::PROXY_AUTHORIZATION,
		HeaderValue::from_static("Basic Zm9v"),
	);
	headers.insert(hyper::header::ACCEPT, HeaderValue::from_static("*/*"));
	let req_ctx = test_request_context("10.0.0.1:1234", headers);

	let mut proxied = hyper::HeaderMap::new();
	proxied.insert(
		hyper::header::CONNECTION,
		HeaderValue::from_static("Upgrade"),
	);
	add_proxy_headers_with_addr(&mut proxied, &req_ctx).expect("add proxy headers");

	assert_eq!(
		proxied.get(hyper::header::CONNECTION).expect("connection"),
		"Upgrade"
	);
	assert!(proxied.get(hyper::header::TRANSFER_ENCODING).is_none());
	assert!(proxied.get(hyper::header::PROXY_AUTHORIZATION).is_none());
	assert_eq!(proxied.get(hyper::header::ACCEPT).expect("accept"), "*/*");
}

#[test]
fn add_proxy_headers_appends_to_an_existing_forwarded_for_chain() {
	let mut headers = hyper::HeaderMap::new();
	headers.append(X_FORWARDED_FOR, HeaderValue::from_static("203.0.113.7"));
	headers.append(X_FORWARDED_FOR, HeaderValue::from_static("198.51.100.4"));
	let req_ctx = test_request_context("10.0.0.1:1234", headers);

	let mut proxied = hyper::HeaderMap::new();
	add_proxy_headers_with_addr(&mut proxied, &req_ctx).expect("add proxy headers");

	let forwarded = proxied
		.get_all(X_FORWARDED_FOR)
		.into_iter()
		.collect::<Vec<_>>();
	assert_eq!(forwarded, vec!["203.0.113.7", "198.51.100.4", "10.0.0.1"]);
}

#[test]
fn add_proxy_headers_does_not_repeat_an_address_already_in_the_chain() {
	let mut headers = hyper::HeaderMap::new();
	headers.insert(
		X_FORWARDED_FOR,
		HeaderValue::from_static("203.0.113.7, 10.0.0.1"),
	);
	let req_ctx = test_request_context("10.0.0.1:1234", headers);

	let mut proxied = hyper::HeaderMap::new();
	add_proxy_headers_with_addr(&mut proxied, &req_ctx).expect("add proxy headers");

	let forwarded = proxied
		.get_all(X_FORWARDED_FOR)
		.into_iter()
		.collect::<Vec<_>>();
	assert_eq!(forwarded, vec!["203.0.113.7, 10.0.0.1"]);
}

#[test]
fn add_proxy_headers_does_not_match_an_address_that_is_only_a_substring() {
	let mut headers = hyper::HeaderMap::new();
	headers.insert(X_FORWARDED_FOR, HeaderValue::from_static("110.0.0.10"));
	let req_ctx = test_request_context("10.0.0.1:1234", headers);

	let mut proxied = hyper::HeaderMap::new();
	add_proxy_headers_with_addr(&mut proxied, &req_ctx).expect("add proxy headers");

	let forwarded = proxied
		.get_all(X_FORWARDED_FOR)
		.into_iter()
		.collect::<Vec<_>>();
	assert_eq!(forwarded, vec!["110.0.0.10", "10.0.0.1"]);
}
