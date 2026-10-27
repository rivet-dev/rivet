mod common;

use common::{PageServer, init_flags};
use sqlite_embedded::database::open_database_from_transport_on_pool;
use sqlite_embedded::query::SqliteStatementError;
use sqlite_embedded::vfs::CommitMode;

/// A syntax error names the SQLite result code and the offset of the token SQLite blamed, so
/// callers can report it the way workerd does.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn syntax_error_reports_its_offset_and_result_code() {
	init_flags();
	let db = open_database_from_transport_on_pool(
		None,
		PageServer::new(),
		"syntax-error".to_string(),
		1,
		tokio::runtime::Handle::current(),
		None,
		CommitMode::Deferred,
	)
	.await
	.expect("open sqlite database");

	let error = db
		.execute("SELEC 2".to_string(), None)
		.await
		.expect_err("syntax error");
	let statement = error
		.downcast_ref::<SqliteStatementError>()
		.unwrap_or_else(|| panic!("not a statement error: {error:?}"));
	assert_eq!(statement.code, libsqlite3_sys::SQLITE_ERROR);
	assert_eq!(statement.offset, Some(0));
	assert_eq!(statement.detail, "near \"SELEC\": syntax error");

	db.close().await.expect("close");
}
