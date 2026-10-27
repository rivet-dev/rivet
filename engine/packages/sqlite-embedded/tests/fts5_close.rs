use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;
use parking_lot::Mutex;
use rivet_envoy_protocol as protocol;
use sqlite_embedded::database::open_database_from_transport;
use sqlite_embedded::query::ColumnValue;
use sqlite_embedded::vfs::SqliteTransport;
use tokio::runtime::Handle;

/// Keeps committed pages in memory so a closed database can be reopened from the same storage.
#[derive(Default)]
struct MemoryTransport {
	state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
	pages: BTreeMap<u32, bytes::Bytes>,
	head_txid: u64,
}

#[async_trait]
impl SqliteTransport for MemoryTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		let state = self.state.lock();
		Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
			protocol::SqliteGetPagesOk {
				pages: request
					.pgnos
					.into_iter()
					.map(|pgno| protocol::SqliteFetchedPage {
						pgno,
						bytes: state.pages.get(&pgno).cloned(),
					})
					.collect(),
				head_txid: Some(state.head_txid),
			},
		))
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		let mut state = self.state.lock();
		for page in request.dirty_pages {
			state.pages.insert(page.pgno, page.bytes);
		}
		state.pages.retain(|pgno, _| *pgno <= request.db_size_pages);
		state.head_txid += 1;
		Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
			protocol::SqliteCommitOk {
				head_txid: Some(state.head_txid),
			},
		))
	}

	async fn commit_stage_begin(
		&self,
		_request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		bail!("memory transport does not stage commits")
	}

	async fn commit_stage_segment(
		&self,
		_request: protocol::SqliteCommitStageSegmentRequest,
	) -> Result<protocol::SqliteCommitStageSegmentResponse> {
		bail!("memory transport does not stage commits")
	}

	async fn commit_finalize(
		&self,
		_request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		bail!("memory transport does not stage commits")
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn database_with_fts5_table_closes_and_reopens_with_its_data() -> Result<()> {
	let transport = Arc::new(MemoryTransport::default());
	let actor_id = "fts5-close-actor".to_string();

	let db = open_database_from_transport(
		transport.clone(),
		actor_id.clone(),
		1,
		Handle::current(),
		None,
	)
	.await?;
	db.exec(
		"CREATE TABLE messages (sequence INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL);
		CREATE VIRTUAL TABLE search USING fts5(text, content='messages', content_rowid='sequence');
		CREATE TRIGGER search_insert AFTER INSERT ON messages BEGIN
			INSERT INTO search(rowid, text) VALUES (new.sequence, new.text);
		END;"
			.to_string(),
	)
	.await?;
	db.exec(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3000)
		INSERT INTO messages (text) SELECT 'message number ' || i FROM n;"
			.to_string(),
	)
	.await?;
	db.query(
		"SELECT count(*) FROM search WHERE search MATCH 'message'".to_string(),
		None,
	)
	.await?;
	db.close_and_wait().await?;
	drop(db);

	let db = open_database_from_transport(transport, actor_id, 2, Handle::current(), None).await?;
	let result = db
		.query(
			"SELECT count(*) FROM search WHERE search MATCH 'message'".to_string(),
			None,
		)
		.await?;
	assert_eq!(result.rows, vec![vec![ColumnValue::Integer(3000)]]);
	db.close_and_wait().await?;
	Ok(())
}
