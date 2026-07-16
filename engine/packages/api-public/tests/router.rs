use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn unsupported_route_method_returns_method_not_allowed() {
	let config = rivet_config::Config::from_root(rivet_config::config::Root::default());
	let pools = rivet_pools::Pools::new(config.clone())
		.await
		.expect("failed to create test pools");
	let app = rivet_api_public::router(config, pools)
		.await
		.expect("failed to create router");
	let server = TestServer::new(app).expect("failed to create test server");

	let response = server.get("/actors/foo").await;

	response.assert_status(StatusCode::METHOD_NOT_ALLOWED);
}
