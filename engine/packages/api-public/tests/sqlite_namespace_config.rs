use axum::http::StatusCode;
use axum_test::TestServer;
use namespace::types::SqliteNamespaceConfig;
use rivet_util::Id;

async fn test_server() -> anyhow::Result<TestServer> {
	let test_deps = rivet_test_deps::TestDeps::new().await?;
	let app = rivet_api_public::router(test_deps.config.clone(), test_deps.pools.clone()).await?;
	Ok(TestServer::new(app)?)
}

fn namespace_id() -> Id {
	Id::new_v1(1)
}

fn enabled_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		default_retention_ms: 3_600_000,
		default_checkpoint_interval_ms: 900_000,
		default_max_checkpoints: 8,
		allow_pitr_read: true,
		allow_pitr_destructive: false,
		allow_pitr_admin: true,
		allow_fork: true,
		pitr_max_bytes_per_actor: 10_000,
		pitr_namespace_budget_bytes: 100_000,
		max_retention_ms: 86_400_000,
		admin_op_rate_per_min: 20,
		concurrent_admin_ops: 3,
		concurrent_forks_per_src: 2,
	}
}

#[tokio::test]
async fn default_namespace_config_returns_disabled() -> anyhow::Result<()> {
	let server = test_server().await?;

	let response = server
		.get(&format!("/namespaces/{}/sqlite-config", namespace_id()))
		.await;

	response.assert_status_ok();
	assert_eq!(response.json::<SqliteNamespaceConfig>(), SqliteNamespaceConfig::default());

	Ok(())
}

#[tokio::test]
async fn put_then_get_roundtrip() -> anyhow::Result<()> {
	let server = test_server().await?;
	let config = enabled_config();
	let path = format!("/namespaces/{}/sqlite-config", namespace_id());

	let response = server.put(&path).json(&config).await;
	response.assert_status_ok();
	assert_eq!(response.json::<SqliteNamespaceConfig>(), config);

	let response = server.get(&path).await;
	response.assert_status_ok();
	assert_eq!(response.json::<SqliteNamespaceConfig>(), config);

	Ok(())
}

#[tokio::test]
async fn put_validates_max_retention() -> anyhow::Result<()> {
	let server = test_server().await?;
	let mut config = enabled_config();
	config.default_retention_ms = config.max_retention_ms + 1;

	let response = server
		.put(&format!("/namespaces/{}/sqlite-config", namespace_id()))
		.json(&config)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);

	Ok(())
}

#[tokio::test]
async fn put_validates_pitr_budget_consistency() -> anyhow::Result<()> {
	let server = test_server().await?;
	let mut config = enabled_config();
	config.pitr_max_bytes_per_actor = config.pitr_namespace_budget_bytes + 1;

	let response = server
		.put(&format!("/namespaces/{}/sqlite-config", namespace_id()))
		.json(&config)
		.await;

	response.assert_status(StatusCode::BAD_REQUEST);

	Ok(())
}
