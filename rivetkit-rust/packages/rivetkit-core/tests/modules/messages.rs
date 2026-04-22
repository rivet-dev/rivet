use super::*;

mod moved_tests {
	use std::collections::HashMap;

	use http::StatusCode;

	use super::{Request, Response};

	#[test]
	fn request_from_parts_round_trips() {
		let request = Request::from_parts(
			"POST",
			"/actors?id=1",
			HashMap::from([("content-type".to_owned(), "application/cbor".to_owned())]),
			vec![1, 2, 3],
		)
		.expect("request should build");

		assert_eq!(request.method(), http::Method::POST);
		assert_eq!(request.uri(), &"/actors?id=1");
		assert_eq!(request.headers()["content-type"], "application/cbor");

		let (method, uri, headers, body) = request.to_parts();
		assert_eq!(method, "POST");
		assert_eq!(uri, "/actors?id=1");
		assert_eq!(
			headers.get("content-type"),
			Some(&"application/cbor".to_owned())
		);
		assert_eq!(body, vec![1, 2, 3]);
	}

	#[test]
	fn response_from_parts_round_trips() {
		let response = Response::from_parts(
			StatusCode::CREATED.as_u16(),
			HashMap::from([("x-test".to_owned(), "ok".to_owned())]),
			b"done".to_vec(),
		)
		.expect("response should build");

		assert_eq!(response.status(), StatusCode::CREATED);
		assert_eq!(response.headers()["x-test"], "ok");

		let (status, headers, body) = response.to_parts();
		assert_eq!(status, StatusCode::CREATED.as_u16());
		assert_eq!(headers.get("x-test"), Some(&"ok".to_owned()));
		assert_eq!(body, b"done");
	}
}
