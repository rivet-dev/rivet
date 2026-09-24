use axum::http::StatusCode;
use axum_test::TestServer;
use utoipa::OpenApi;

#[test]
fn jwt_token_routes_are_root_scoped() {
	let openapi = rivet_api_public::router::ApiDoc::openapi();
	assert!(openapi.paths.paths.contains_key("/auth/tokens"));
	assert!(openapi.paths.paths.contains_key("/auth/tokens/inspect"));
	assert!(
		!openapi
			.paths
			.paths
			.contains_key("/namespaces/{namespace}/auth/tokens")
	);
}

#[tokio::test]
async fn router_preserves_method_handling_and_public_health() {
	let mut root = rivet_config::config::Root::default();
	root.auth = Some(rivet_config::config::Auth {
		admin_token: rivet_config::secret::Secret::new("router-test-admin".into()),
		insecure_allow_unauthenticated: false,
		jwt: rivet_config::config::Jwt {
			enabled: Some(false),
			..Default::default()
		},
	});
	let config = rivet_config::Config::from_root(root);
	let pools = rivet_pools::Pools::new(config.clone())
		.await
		.expect("failed to create test pools");
	let app = rivet_api_public::router(config, pools, None)
		.await
		.expect("failed to create router");
	let server = TestServer::new(app).expect("failed to create test server");

	server
		.get("/actors/foo")
		.await
		.assert_status(StatusCode::METHOD_NOT_ALLOWED);
	server.get("/health").await.assert_status_ok();
}
