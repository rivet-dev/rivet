//! CORS for gateway requests. An actor owns the CORS policy of its own responses, so the gateway
//! only answers preflights, which must not wait on a lookup, wake or auth, and fills in defaults
//! on responses that carry none, such as its own errors. RivetKit clients send every request
//! with credentials, which rejects wildcards, so every value echoes the request.

use axum::{
	body::Body,
	extract::Request,
	middleware::Next,
	response::Response,
};
use hyper::{
	Method, StatusCode,
	header::{
		ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
		ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS,
		ACCESS_CONTROL_MAX_AGE, ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD,
		HeaderValue, ORIGIN, RETRY_AFTER, VARY,
	},
};
use rivet_guard_core::{X_RIVET_RAY_ID, errors::X_RIVET_ERROR};

const PREFLIGHT_MAX_AGE_SECS: u64 = 86_400;

/// Answers a preflight without running the request, and otherwise adds default CORS headers
/// to a response unless it already set its own.
///
/// The preflight allows everything the browser asks for. The defaults allow the request's
/// origin with credentials and expose the headers clients read from the engine. A request
/// without an `Origin` is not a CORS request and gets no defaults.
pub(crate) async fn cors(request: Request, next: Next) -> Response {
	if is_preflight(&request) {
		return preflight_response(&request);
	}

	let origin = request.headers().get(ORIGIN).cloned();
	let mut response = next.run(request).await;

	let headers = response.headers_mut();
	if let Some(origin) = origin
		&& !headers.contains_key(ACCESS_CONTROL_ALLOW_ORIGIN)
	{
		headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
		headers.insert(
			ACCESS_CONTROL_ALLOW_CREDENTIALS,
			HeaderValue::from_static("true"),
		);
		// The headers the engine adds that clients read. A credentialed request does not
		// accept `*`.
		let exposed = [
			HeaderValue::from_static(X_RIVET_RAY_ID),
			HeaderValue::from_static(X_RIVET_ERROR),
			HeaderValue::from_name(RETRY_AFTER),
		];
		for value in exposed {
			headers.append(ACCESS_CONTROL_EXPOSE_HEADERS, value);
		}
		// The response depends on the origin it echoes, so caches must key on it too.
		headers.append(VARY, HeaderValue::from_static("origin"));
	}

	response
}

fn is_preflight(request: &Request) -> bool {
	request.method() == Method::OPTIONS
		&& request.headers().contains_key(ACCESS_CONTROL_REQUEST_METHOD)
}

fn preflight_response(request: &Request) -> Response {
	let mut response = Response::new(Body::empty());
	*response.status_mut() = StatusCode::NO_CONTENT;

	// A credentialed preflight rejects wildcards, so echo what the browser asked for.
	let echoed = [
		(ORIGIN, ACCESS_CONTROL_ALLOW_ORIGIN),
		(ACCESS_CONTROL_REQUEST_METHOD, ACCESS_CONTROL_ALLOW_METHODS),
		(ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_ALLOW_HEADERS),
	];
	let headers = response.headers_mut();
	for (request_header, response_header) in echoed {
		if let Some(value) = request.headers().get(&request_header) {
			headers.insert(response_header, value.clone());
		}
	}

	headers.insert(
		ACCESS_CONTROL_ALLOW_CREDENTIALS,
		HeaderValue::from_static("true"),
	);
	headers.insert(
		ACCESS_CONTROL_MAX_AGE,
		HeaderValue::from(PREFLIGHT_MAX_AGE_SECS),
	);
	headers.insert(
		VARY,
		HeaderValue::from_static(
			"origin, access-control-request-method, access-control-request-headers",
		),
	);

	response
}
