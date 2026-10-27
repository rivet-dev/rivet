mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{HEAD_STEP, INITIAL_HEAD, PageServer, init_flags};
use sqlite_embedded::database::{NativeDatabaseHandle, open_database_from_transport_on_pool};
use sqlite_embedded::vfs::{CommitMode, DatabaseFailure, FlushError};
use sqlite_types::ColumnValue;

const WAIT: Duration = Duration::from_secs(10);

async fn open(
	server: &Arc<PageServer>,
	actor_id: &str,
	generation: u64,
	commit_mode: CommitMode,
) -> NativeDatabaseHandle {
	init_flags();
	open_database_from_transport_on_pool(
		None,
		server.clone(),
		actor_id.to_string(),
		generation,
		tokio::runtime::Handle::current(),
		None,
		commit_mode,
	)
	.await
	.expect("open sqlite database")
}

async fn exec(db: &NativeDatabaseHandle, sql: &str) {
	db.execute(sql.to_string(), None)
		.await
		.unwrap_or_else(|error| panic!("`{sql}` failed: {error:?}"));
}

async fn integer(db: &NativeDatabaseHandle, sql: &str) -> i64 {
	let result = db.query(sql.to_string(), None).await.expect("query");
	match result.rows.first().and_then(|row| row.first()) {
		Some(ColumnValue::Integer(value)) => *value,
		other => panic!("`{sql}` returned {other:?}"),
	}
}

async fn wait_durable(db: &NativeDatabaseHandle) -> Result<(), FlushError> {
	tokio::time::timeout(WAIT, db.wait_for_flush(db.commit_seq()))
		.await
		.expect("flush wait timed out")
}

/// The 3.x engine moves the head by the filesystem sequence of each commit, not by one. Every
/// commit still becomes durable, in order, and a later generation reads all of them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn commits_become_durable_when_each_commit_moves_the_head_by_more_than_one() {
	let server = PageServer::new();
	let db = open(&server, "head-step", 1, CommitMode::Deferred).await;
	exec(&db, "CREATE TABLE t (v INTEGER)").await;
	for value in 0..20 {
		exec(&db, &format!("INSERT INTO t VALUES ({value})")).await;
		if value % 5 == 0 {
			wait_durable(&db).await.expect("commit becomes durable");
		}
	}
	wait_durable(&db).await.expect("commits become durable");
	assert!(server.head().await >= INITIAL_HEAD + 2 * HEAD_STEP);
	db.close().await.expect("close drains held commits");

	let reopened = open(&server, "head-step", 2, CommitMode::Awaited).await;
	assert_eq!(integer(&reopened, "SELECT count(*) FROM t").await, 20);
	assert_eq!(integer(&reopened, "SELECT sum(v) FROM t").await, 190);
	reopened.close().await.expect("close");
}

/// A flush whose reply is lost after it applied is retried with the same fence. The retry
/// recognizes its own commit without assuming the head moved by one, so nothing diverges and
/// later commits keep flushing on top of the head the engine reports.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flush_with_a_lost_reply_is_recognized_and_later_commits_continue() {
	let server = PageServer::new();
	let db = open(&server, "lost-reply", 1, CommitMode::Deferred).await;
	exec(&db, "CREATE TABLE t (v INTEGER)").await;
	wait_durable(&db).await.expect("schema becomes durable");

	server.lose_next_reply().await;
	exec(&db, "INSERT INTO t VALUES (1)").await;
	wait_durable(&db)
		.await
		.expect("the applied flush is recognized");
	exec(&db, "INSERT INTO t VALUES (2)").await;
	wait_durable(&db)
		.await
		.expect("the next flush is fenced on the real head");
	db.close().await.expect("close");

	let reopened = open(&server, "lost-reply", 2, CommitMode::Awaited).await;
	assert_eq!(integer(&reopened, "SELECT count(*) FROM t").await, 2);
	reopened.close().await.expect("close");
}

/// A read that misses the caches while a flush is in flight carries no fence. The engine may
/// already have applied the flush and report its new head, several steps past the durable one,
/// before the flush's reply arrives. The read succeeds and the database keeps working.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cold_read_during_an_applied_flush_accepts_the_engine_head() {
	let server = PageServer::new();
	let setup = open(&server, "cold-read", 1, CommitMode::Awaited).await;
	exec(&setup, "CREATE TABLE big (b BLOB)").await;
	exec(&setup, "CREATE TABLE t (v INTEGER)").await;
	exec(
		&setup,
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 200) \
		 INSERT INTO big SELECT randomblob(1000) FROM n",
	)
	.await;
	setup.close().await.expect("close");

	let db = open(&server, "cold-read", 2, CommitMode::Deferred).await;
	let applied = server.applied_commits();
	server.hold_replies(true);
	exec(&db, "INSERT INTO t VALUES (1)").await;
	tokio::time::timeout(WAIT, server.wait_for_applied_commits(applied + 1))
		.await
		.expect("flush applies");

	// The flush applied but its reply is held, so the engine's head is past the durable one.
	assert_eq!(integer(&db, "SELECT count(*) FROM big").await, 200);
	assert_eq!(
		integer(&db, "SELECT sum(length(b)) FROM big").await,
		200 * 1000
	);

	server.hold_replies(false);
	wait_durable(&db).await.expect("held flush becomes durable");
	exec(&db, "INSERT INTO t VALUES (2)").await;
	wait_durable(&db).await.expect("next flush becomes durable");
	db.close().await.expect("close");

	let reopened = open(&server, "cold-read", 3, CommitMode::Awaited).await;
	assert_eq!(integer(&reopened, "SELECT count(*) FROM t").await, 2);
	reopened.close().await.expect("close");
}

/// When another writer moves the head, held commits can never become durable. Waiters reject,
/// even for a ticket that was durable before, the generation reports the failure, and later
/// statements fail instead of building on lost writes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_writer_breaks_the_database_and_fails_waiters() {
	let server = PageServer::new();
	let db = open(&server, "diverged", 1, CommitMode::Deferred).await;
	exec(&db, "CREATE TABLE t (v INTEGER)").await;
	wait_durable(&db).await.expect("schema becomes durable");
	let durable_ticket = db.commit_seq();

	server.write_as_another_writer().await;
	// The write fails on a fenced read or commits locally and fails to flush. Either way the
	// failure reaches every waiter.
	let _ = db
		.execute("INSERT INTO t VALUES (1)".to_string(), None)
		.await;

	let failure = tokio::time::timeout(WAIT, db.wait_for_failure())
		.await
		.expect("failure is reported");
	assert!(
		matches!(failure, DatabaseFailure::Flush(_)),
		"unexpected failure {failure:?}"
	);
	assert!(db.wait_for_flush(durable_ticket).await.is_err());
	assert!(
		db.execute("INSERT INTO t VALUES (2)".to_string(), None)
			.await
			.is_err()
	);
	assert!(db.close().await.is_err());
}

/// A flush the engine never accepts fails its waiters once the retry deadline passes, and the
/// database stops accepting writes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flush_that_never_succeeds_fails_waiters_after_the_retry_deadline() {
	let server = PageServer::new();
	let db = open(&server, "unreachable", 1, CommitMode::Deferred).await;
	exec(&db, "CREATE TABLE t (v INTEGER)").await;
	wait_durable(&db).await.expect("schema becomes durable");

	server.fail_commits().await;
	exec(&db, "INSERT INTO t VALUES (1)").await;
	let error = wait_durable(&db).await.expect_err("flush fails");
	assert!(
		matches!(error, FlushError::RetryDeadlineExceeded { .. }),
		"unexpected error {error:?}"
	);
	assert!(
		matches!(
			tokio::time::timeout(WAIT, db.wait_for_failure()).await,
			Ok(DatabaseFailure::Flush(_))
		),
		"failure is reported"
	);
	assert!(
		db.execute("INSERT INTO t VALUES (2)".to_string(), None)
			.await
			.is_err()
	);
}

/// A ticket past the newest local commit names nothing this generation wrote.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ticket_ahead_of_every_commit_is_rejected() {
	let server = PageServer::new();
	let db = open(&server, "ticket-ahead", 1, CommitMode::Deferred).await;
	exec(&db, "CREATE TABLE t (v INTEGER)").await;
	let error = db
		.wait_for_flush(db.commit_seq() + 1)
		.await
		.expect_err("ticket is ahead");
	assert!(matches!(error, FlushError::InvalidSequence { .. }));
	db.close().await.expect("close");
}
