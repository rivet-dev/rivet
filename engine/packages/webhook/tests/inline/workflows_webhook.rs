use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

use super::*;

fn retry_after(value: &str) -> HeaderMap {
	let mut headers = HeaderMap::new();
	headers.insert(
		RETRY_AFTER,
		HeaderValue::from_str(value).expect("invalid header value"),
	);
	headers
}

// MARK: is_retryable

#[test]
fn retries_transient_failures() {
	assert!(is_retryable(&errors::Webhook::RequestFailed {
		reason: "connection refused".to_string(),
	}));
	assert!(is_retryable(&errors::Webhook::DeliveryFailed {
		status: 429
	}));
	assert!(is_retryable(&errors::Webhook::DeliveryFailed {
		status: 500
	}));
	assert!(is_retryable(&errors::Webhook::DeliveryFailed {
		status: 503
	}));
}

#[test]
fn does_not_retry_permanent_failures() {
	assert!(!is_retryable(&errors::Webhook::DeliveryFailed {
		status: 400
	}));
	assert!(!is_retryable(&errors::Webhook::DeliveryFailed {
		status: 404
	}));
	assert!(!is_retryable(&errors::Webhook::DestinationBlocked {
		reason: "loopback".to_string(),
	}));
}

// MARK: Backoff

#[test]
fn backoff_doubles_and_caps() {
	assert_eq!(delivery_backoff(0), Duration::from_secs(5));
	assert_eq!(delivery_backoff(1), Duration::from_secs(10));
	assert_eq!(delivery_backoff(2), Duration::from_secs(20));
	assert_eq!(delivery_backoff(6), Duration::from_secs(300));
	assert_eq!(delivery_backoff(100), Duration::from_secs(300));
}

#[test]
fn next_delay_uses_backoff_without_retry_after() {
	assert_eq!(next_delivery_delay(None, 1), delivery_backoff(1));
}

#[test]
fn next_delay_clamps_retry_after() {
	assert_eq!(next_delivery_delay(Some(0), 1), MIN_RETRY_AFTER);
	assert_eq!(
		next_delivery_delay(Some(30_000), 1),
		Duration::from_secs(30)
	);
	assert_eq!(next_delivery_delay(Some(10_000_000), 1), MAX_RETRY_AFTER);
}

// MARK: parse_retry_after

#[test]
fn parses_retry_after_seconds() {
	assert_eq!(parse_retry_after(&retry_after("120")), Some(120_000));
}

#[test]
fn parses_retry_after_past_date_as_zero() {
	assert_eq!(
		parse_retry_after(&retry_after("Wed, 21 Oct 2015 07:28:00 GMT")),
		Some(0)
	);
}

#[test]
fn parses_retry_after_future_date() {
	let deadline = chrono::Utc::now() + chrono::Duration::seconds(60);
	let delay =
		parse_retry_after(&retry_after(&deadline.to_rfc2822())).expect("future date should parse");
	assert!(
		delay > 50_000 && delay <= 60_000,
		"unexpected delay {delay}"
	);
}

#[test]
fn ignores_invalid_retry_after() {
	assert_eq!(parse_retry_after(&retry_after("soon")), None);
	assert_eq!(parse_retry_after(&HeaderMap::new()), None);
}
