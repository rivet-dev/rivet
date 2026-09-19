use hyper::header::HeaderValue;

use super::*;

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
