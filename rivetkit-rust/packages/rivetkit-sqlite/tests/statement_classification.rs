use std::ffi::CString;
use std::ptr;

use libsqlite3_sys::{SQLITE_OK, sqlite3, sqlite3_close, sqlite3_open};
use rivetkit_sqlite::query::{
	StatementAuthorizerActionKind, classify_statement, exec_statements,
};

struct MemoryDb(*mut sqlite3);

impl MemoryDb {
	fn open() -> Self {
		let name = CString::new(":memory:").unwrap();
		let mut db = ptr::null_mut();
		let rc = unsafe { sqlite3_open(name.as_ptr(), &mut db) };
		assert_eq!(rc, SQLITE_OK);
		Self(db)
	}

	fn as_ptr(&self) -> *mut sqlite3 {
		self.0
	}
}

impl Drop for MemoryDb {
	fn drop(&mut self) {
		unsafe {
			sqlite3_close(self.0);
		}
	}
}

#[test]
fn select_is_reader_eligible() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "SELECT 1 AS value").unwrap();

	assert!(classification.has_statement);
	assert!(classification.sqlite_readonly);
	assert!(!classification.has_trailing_sql);
	assert!(classification.reader_eligible());
	assert!(
		classification
			.authorizer
			.actions
			.iter()
			.any(|action| action.kind == StatementAuthorizerActionKind::Select)
	);
}

#[test]
fn readonly_pragma_is_reader_eligible_and_captures_pragma_usage() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "PRAGMA user_version").unwrap();

	assert!(classification.sqlite_readonly);
	assert!(classification.reader_eligible());
	assert!(classification.authorizer.pragma_usage);
}

#[test]
fn mutating_pragma_is_not_reader_eligible() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "PRAGMA user_version = 7").unwrap();

	assert!(!classification.sqlite_readonly);
	assert!(!classification.reader_eligible());
	assert!(classification.authorizer.pragma_usage);
}

#[test]
fn insert_returning_is_a_write_operation() {
	let db = MemoryDb::open();
	exec_statements(
		db.as_ptr(),
		"CREATE TABLE items(id INTEGER PRIMARY KEY, label TEXT);",
	)
	.unwrap();

	let classification = classify_statement(
		db.as_ptr(),
		"INSERT INTO items(label) VALUES ('alpha') RETURNING id",
	)
	.unwrap();

	assert!(!classification.sqlite_readonly);
	assert!(!classification.reader_eligible());
	assert!(classification.authorizer.write_operations);
	assert!(
		classification
			.authorizer
			.actions
			.iter()
			.any(|action| action.kind == StatementAuthorizerActionKind::Insert)
	);
}

#[test]
fn cte_insert_returning_is_a_write_operation() {
	let db = MemoryDb::open();
	exec_statements(db.as_ptr(), "CREATE TABLE items(value INTEGER);").unwrap();

	let classification = classify_statement(
		db.as_ptr(),
		"WITH source(value) AS (VALUES (1)) INSERT INTO items(value) SELECT value FROM source RETURNING value",
	)
	.unwrap();

	assert!(!classification.sqlite_readonly);
	assert!(!classification.reader_eligible());
	assert!(classification.authorizer.write_operations);
}

#[test]
fn vacuum_is_not_reader_eligible() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "VACUUM").unwrap();

	assert!(!classification.sqlite_readonly);
	assert!(!classification.reader_eligible());
}

#[test]
fn attach_is_not_reader_eligible_and_captures_attach() {
	let db = MemoryDb::open();
	let classification =
		classify_statement(db.as_ptr(), "ATTACH DATABASE ':memory:' AS attached").unwrap();

	assert!(!classification.reader_eligible());
	assert!(classification.authorizer.attach);
}

#[test]
fn begin_is_not_reader_eligible_and_captures_transaction_control() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "BEGIN").unwrap();

	assert!(classification.sqlite_readonly);
	assert!(!classification.reader_eligible());
	assert!(classification.authorizer.transaction_control);
	assert!(
		classification
			.authorizer
			.actions
			.iter()
			.any(|action| action.kind == StatementAuthorizerActionKind::Transaction)
	);
}

#[test]
fn savepoint_is_not_reader_eligible_and_captures_transaction_control() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "SAVEPOINT manual").unwrap();

	assert!(classification.sqlite_readonly);
	assert!(!classification.reader_eligible());
	assert!(classification.authorizer.transaction_control);
	assert!(
		classification
			.authorizer
			.actions
			.iter()
			.any(|action| action.kind == StatementAuthorizerActionKind::Savepoint)
	);
}

#[test]
fn multi_statement_sql_is_not_reader_eligible() {
	let db = MemoryDb::open();
	let classification = classify_statement(db.as_ptr(), "SELECT 1; SELECT 2").unwrap();

	assert!(classification.sqlite_readonly);
	assert!(classification.has_trailing_sql);
	assert!(!classification.reader_eligible());
}
