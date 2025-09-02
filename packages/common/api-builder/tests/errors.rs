use anyhow::*;
use axum_test::TestServer;
use rivet_api_builder::error_response::ErrorResponse;
use rivet_api_builder::{create_router, prelude::*};
use rivet_error::*;

// Custom error type for testing
#[derive(RivetError, Debug)]
#[error("test", "test_error", "Test error")]
pub struct TestError;

// Wrapped custom error
#[derive(thiserror::Error, Debug)]
enum TestErrorWrapper {
	#[error("wrapped error")]
	WrapError(#[source] anyhow::Error),
}

// Handler that returns invalid token error
async fn handle_invalid_token(_ctx: ApiCtx, _path: (), _query: ()) -> Result<()> {
	Err(rivet_api_builder::errors::ApiInvalidToken.build())
}

// Handler that returns unauthorized error
async fn handle_unauthorized(_ctx: ApiCtx, _path: (), _query: ()) -> Result<()> {
	Err(rivet_api_builder::errors::ApiUnauthorized.build())
}

// Handler that returns forbidden error
async fn handle_forbidden(_ctx: ApiCtx, _path: (), _query: ()) -> Result<()> {
	Err(rivet_api_builder::errors::ApiForbidden.build())
}

// Handler that returns internal error (anyhow bail)
async fn handle_internal_error(_ctx: ApiCtx, _path: (), _query: ()) -> Result<()> {
	bail!("Something went wrong internally")
}

// Handler that returns custom error type
async fn handle_custom_error(_ctx: ApiCtx, _path: (), _query: ()) -> Result<()> {
	Err(TestError.build())
}

// Handler that returns custom error type wrapped in another anyhow error
async fn handle_custom_error_wrapped(_ctx: ApiCtx, _path: (), _query: ()) -> Result<()> {
	Err(TestErrorWrapper::WrapError(TestError.build()).into())
}

#[tokio::test]
async fn test_error_responses() {
	let config = rivet_config::Config::from_root(rivet_config::config::Root::default());
	let pools = rivet_pools::Pools::new(config.clone())
		.await
		.expect("Failed to create test pools");

	// Create router with error routes
	let app = create_router("test", config, pools, |router| {
		router
			.route("/invalid-token", get(handle_invalid_token))
			.route("/unauthorized", get(handle_unauthorized))
			.route("/forbidden", get(handle_forbidden))
			.route("/internal-error", get(handle_internal_error))
			.route("/custom-error", get(handle_custom_error))
			.route("/custom-error-wrapped", get(handle_custom_error_wrapped))
	})
	.await
	.expect("Failed to create router");

	let server = TestServer::new(app).unwrap();

	// Test invalid token error
	let res = server.get("/invalid-token").await;
	res.assert_status(axum::http::StatusCode::UNAUTHORIZED);
	let error_response: ErrorResponse = res.json();
	assert_eq!(error_response.group, "api");
	assert_eq!(
		error_response.message,
		"The provided authentication token is invalid"
	);

	// Test unauthorized error
	let res = server.get("/unauthorized").await;
	res.assert_status(axum::http::StatusCode::UNAUTHORIZED);
	let error_response: ErrorResponse = res.json();
	assert_eq!(error_response.group, "api");
	assert_eq!(error_response.code, "unauthorized");
	assert_eq!(error_response.message, "Authentication required");

	// Test forbidden error
	let res = server.get("/forbidden").await;
	res.assert_status(axum::http::StatusCode::FORBIDDEN);
	let error_response: ErrorResponse = res.json();
	assert_eq!(error_response.group, "api");
	assert_eq!(error_response.code, "forbidden");
	assert_eq!(error_response.message, "Access denied");

	// Test internal error (anyhow bail)
	let res = server.get("/internal-error").await;
	res.assert_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
	let error_response: ErrorResponse = res.json();
	assert_eq!(error_response.group, "core");
	assert_eq!(error_response.code, "internal_error");
	assert_eq!(error_response.message, "An internal error occurred");

	// Test custom error type
	let res = server.get("/custom-error").await;
	res.assert_status(axum::http::StatusCode::BAD_REQUEST);
	let error_response: ErrorResponse = res.json();
	assert_eq!(error_response.group, "test");
	assert_eq!(error_response.code, "test_error");
	assert_eq!(error_response.message, "Test error");

	// Test wrapped custom error type
	let res = server.get("/custom-error-wrapped").await;
	res.assert_status(axum::http::StatusCode::BAD_REQUEST);
	let error_response: ErrorResponse = res.json();
	assert_eq!(error_response.group, "test");
	assert_eq!(error_response.code, "test_error");
	assert_eq!(error_response.message, "Test error");
}
