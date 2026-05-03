use anyhow::Result;
use axum_test::TestServer;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use rivet_config::config::{Database, Root, db::FileSystem};

#[tokio::test]
async fn depot_inspect_routes_are_registered_on_api_peer() -> Result<()> {
	let tempdir = tempfile::tempdir()?;
	let mut root = Root::default();
	root.database = Some(Database::FileSystem(FileSystem {
		path: tempdir.path().join("udb"),
	}));
	let config = rivet_config::Config::from_root(root);
	let pools = rivet_pools::Pools::test(config.clone()).await?;
	let app = rivet_api_peer::create_router(config, pools).await?;
	let server = TestServer::new(app)?;

	let res = server.get("/depot/inspect/summary").await;
	res.assert_status_ok();

	let key = BASE64_URL_SAFE_NO_PAD.encode([depot::keys::SQLITE_SUBSPACE_PREFIX]);
	let res = server
		.get(&format!("/depot/inspect/raw/decode-key/{key}"))
		.await;
	res.assert_status_ok();

	let branch_id = uuid::Uuid::nil();
	let res = server
		.get(&format!("/depot/inspect/branches/{branch_id}/rows/commits"))
		.add_query_param("limit", "1")
		.await;
	res.assert_status_ok();

	Ok(())
}
