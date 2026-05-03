use std::ffi::CString;
use std::ptr;

use depot_client::query::{
	BindParam, ColumnValue, exec_statements, execute_statement, query_statement,
};
use libsqlite3_sys::{SQLITE_OK, sqlite3, sqlite3_close, sqlite3_open};

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
fn query_sql_with_embedded_nul_is_rejected() {
	let db = MemoryDb::open();
	let err = query_statement(db.as_ptr(), "SELECT 1\0; --", None)
		.expect_err("SQL strings with embedded NUL bytes should be rejected");

	let chain = err
		.chain()
		.map(|err| err.to_string())
		.collect::<Vec<_>>()
		.join(": ");
	assert!(
		chain.to_lowercase().contains("nul byte"),
		"expected CString embedded NUL error, got {chain:?}"
	);
}

#[test]
fn text_with_embedded_nul_round_trips() {
	let db = MemoryDb::open();
	exec_statements(
		db.as_ptr(),
		"CREATE TABLE items(id INTEGER PRIMARY KEY, label TEXT);",
	)
	.unwrap();

	execute_statement(
		db.as_ptr(),
		"INSERT INTO items(label) VALUES (?);",
		Some(&[BindParam::Text("a\0b".to_owned())]),
	)
	.unwrap();

	let rows = query_statement(
		db.as_ptr(),
		"SELECT label, hex(label), length(label) FROM items;",
		None,
	)
	.unwrap();

	assert_eq!(
		rows.rows,
		vec![vec![
			ColumnValue::Text("a\0b".to_owned()),
			ColumnValue::Text("610062".to_owned()),
			ColumnValue::Integer(1),
		]]
	);
}
