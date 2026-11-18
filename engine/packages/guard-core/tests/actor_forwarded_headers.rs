use hyper::HeaderMap;
use rivet_guard_core::utils::actor_forwarded_headers;

#[test]
fn drops_x_rivet_token_header() {
	let mut headers = HeaderMap::new();
	headers.insert("x-rivet-token", "engine-secret".parse().unwrap());
	headers.insert("content-type", "application/json".parse().unwrap());

	let forwarded = actor_forwarded_headers(&headers);

	assert!(forwarded.get("x-rivet-token").is_none());
	assert_eq!(forwarded["content-type"], "application/json");
}

#[test]
fn keeps_application_authorization_header() {
	let mut headers = HeaderMap::new();
	headers.insert("x-rivet-token", "engine-secret".parse().unwrap());
	headers.insert("authorization", "Bearer app-session".parse().unwrap());

	let forwarded = actor_forwarded_headers(&headers);

	assert_eq!(forwarded["authorization"], "Bearer app-session");
	assert!(forwarded.get("x-rivet-token").is_none());
}

#[test]
fn strips_token_subprotocol_and_keeps_the_rest() {
	let mut headers = HeaderMap::new();
	headers.insert(
		"sec-websocket-protocol",
		"rivet, rivet_encoding.bare, rivet_token.engine-secret"
			.parse()
			.unwrap(),
	);

	let forwarded = actor_forwarded_headers(&headers);

	assert_eq!(
		forwarded["sec-websocket-protocol"],
		"rivet, rivet_encoding.bare"
	);
}

#[test]
fn strips_token_subprotocol_in_any_position() {
	let mut headers = HeaderMap::new();
	headers.insert(
		"sec-websocket-protocol",
		"rivet_token.engine-secret,rivet,  rivet_actor.abc ,custom"
			.parse()
			.unwrap(),
	);

	let forwarded = actor_forwarded_headers(&headers);

	assert_eq!(
		forwarded["sec-websocket-protocol"],
		"rivet, rivet_actor.abc, custom"
	);
	assert!(!forwarded["sec-websocket-protocol"].contains("engine-secret"));
}

#[test]
fn leaves_subprotocols_without_token_unchanged() {
	let mut headers = HeaderMap::new();
	headers.insert(
		"sec-websocket-protocol",
		"rivet, rivet_encoding.json".parse().unwrap(),
	);

	let forwarded = actor_forwarded_headers(&headers);

	assert_eq!(
		forwarded["sec-websocket-protocol"],
		"rivet, rivet_encoding.json"
	);
}
