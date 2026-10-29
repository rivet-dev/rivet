use std::sync::Arc;

use anyhow::Result;
use depot::conveyer::Db;
use depot_client_embedded::open_database_from_embedded_depot;
use gas::prelude::Id;
use rivet_pools::NodeId;

async fn head_txid(db: &Db) -> Result<u64> {
	Ok(db.get_pages_with_metadata(vec![1]).await?.head_txid)
}

#[tokio::test(flavor = "multi_thread")]
async fn reopening_a_database_commits_nothing() -> Result<()> {
	let dir = tempfile::tempdir()?;
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(dir.path().to_path_buf()).await?;
	let udb = Arc::new(universaldb::Database::new(Arc::new(driver)));
	let actor_id = "reopen-actor".to_string();
	let db = Arc::new(Db::new(udb, Id::new_v1(1), actor_id.clone(), NodeId::new()));

	let first = open_database_from_embedded_depot(
		Arc::clone(&db),
		actor_id.clone(),
		1,
		tokio::runtime::Handle::current(),
		None,
	)
	.await?;
	first
		.exec("CREATE TABLE items (id INTEGER PRIMARY KEY); INSERT INTO items VALUES (1);".into())
		.await?;
	first.close_and_wait().await?;
	let head_before = head_txid(&db).await?;

	// Every actor wake reopens its database. Opening must not write.
	let reopened = open_database_from_embedded_depot(
		Arc::clone(&db),
		actor_id,
		2,
		tokio::runtime::Handle::current(),
		None,
	)
	.await?;
	assert_eq!(head_txid(&db).await?, head_before);
	reopened.close_and_wait().await?;
	Ok(())
}
