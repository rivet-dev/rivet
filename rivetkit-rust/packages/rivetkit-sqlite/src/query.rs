use std::ffi::{CStr, CString, c_char};
use std::ptr;

use anyhow::{Result, anyhow};
use libsqlite3_sys::{
	SQLITE_BLOB, SQLITE_DONE, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW,
	SQLITE_TEXT, SQLITE_TRANSIENT, sqlite3, sqlite3_bind_blob, sqlite3_bind_double,
	sqlite3_bind_int64, sqlite3_bind_null, sqlite3_bind_text, sqlite3_changes, sqlite3_column_blob,
	sqlite3_column_bytes, sqlite3_column_count, sqlite3_column_double, sqlite3_column_int64,
	sqlite3_column_name, sqlite3_column_text, sqlite3_column_type, sqlite3_errmsg,
	sqlite3_finalize, sqlite3_prepare_v2, sqlite3_step,
};

#[derive(Clone, Debug, PartialEq)]
pub enum BindParam {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecResult {
	pub changes: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

pub fn execute_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[BindParam]>,
) -> Result<ExecResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, "failed to prepare sqlite statement"));
	}
	if stmt.is_null() {
		return Ok(ExecResult { changes: 0 });
	}

	let result = (|| {
		if let Some(params) = params {
			bind_params(db, stmt, params)?;
		}

		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				return Err(sqlite_error(db, "failed to execute sqlite statement"));
			}
		}

		Ok(ExecResult {
			changes: unsafe { sqlite3_changes(db) as i64 },
		})
	})();

	unsafe {
		sqlite3_finalize(stmt);
	}

	result
}

pub fn query_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[BindParam]>,
) -> Result<QueryResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, "failed to prepare sqlite query"));
	}
	if stmt.is_null() {
		return Ok(QueryResult {
			columns: Vec::new(),
			rows: Vec::new(),
		});
	}

	let result = (|| {
		if let Some(params) = params {
			bind_params(db, stmt, params)?;
		}

		let columns = collect_columns(stmt);
		let mut rows = Vec::new();

		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				return Err(sqlite_error(db, "failed to step sqlite query"));
			}

			let mut row = Vec::with_capacity(columns.len());
			for index in 0..columns.len() {
				row.push(column_value(stmt, index as i32));
			}
			rows.push(row);
		}

		Ok(QueryResult { columns, rows })
	})();

	unsafe {
		sqlite3_finalize(stmt);
	}

	result
}

pub fn exec_statements(db: *mut sqlite3, sql: &str) -> Result<QueryResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut remaining = c_sql.as_ptr();
	let mut final_result = QueryResult {
		columns: Vec::new(),
		rows: Vec::new(),
	};

	while unsafe { *remaining } != 0 {
		let mut stmt = ptr::null_mut();
		let mut tail = ptr::null();
		let rc = unsafe { sqlite3_prepare_v2(db, remaining, -1, &mut stmt, &mut tail) };
		if rc != SQLITE_OK {
			return Err(sqlite_error(db, "failed to prepare sqlite exec statement"));
		}

		if stmt.is_null() {
			if tail == remaining {
				break;
			}
			remaining = tail;
			continue;
		}

		let result = (|| {
			let columns = collect_columns(stmt);
			let mut rows = Vec::new();
			loop {
				let step_rc = unsafe { sqlite3_step(stmt) };
				if step_rc == SQLITE_DONE {
					break;
				}
				if step_rc != SQLITE_ROW {
					return Err(sqlite_error(db, "failed to step sqlite exec statement"));
				}

				let mut row = Vec::with_capacity(columns.len());
				for index in 0..columns.len() {
					row.push(column_value(stmt, index as i32));
				}
				rows.push(row);
			}

			Ok((columns, rows))
		})();

		unsafe {
			sqlite3_finalize(stmt);
		}

		let (columns, rows) = result?;
		if !columns.is_empty() || !rows.is_empty() {
			final_result = QueryResult { columns, rows };
		}

		if tail == remaining {
			break;
		}
		remaining = tail;
	}

	Ok(final_result)
}

fn bind_params(
	db: *mut sqlite3,
	stmt: *mut libsqlite3_sys::sqlite3_stmt,
	params: &[BindParam],
) -> Result<()> {
	for (index, param) in params.iter().enumerate() {
		let bind_index = (index + 1) as i32;
		let rc = match param {
			BindParam::Null => unsafe { sqlite3_bind_null(stmt, bind_index) },
			BindParam::Integer(value) => unsafe { sqlite3_bind_int64(stmt, bind_index, *value) },
			BindParam::Float(value) => unsafe { sqlite3_bind_double(stmt, bind_index, *value) },
			BindParam::Text(value) => {
				let text = CString::new(value.as_str()).map_err(|err| anyhow!(err.to_string()))?;
				unsafe {
					sqlite3_bind_text(stmt, bind_index, text.as_ptr(), -1, SQLITE_TRANSIENT())
				}
			}
			BindParam::Blob(value) => unsafe {
				sqlite3_bind_blob(
					stmt,
					bind_index,
					value.as_ptr() as *const _,
					value.len() as i32,
					SQLITE_TRANSIENT(),
				)
			},
		};

		if rc != SQLITE_OK {
			return Err(sqlite_error(db, "failed to bind sqlite parameter"));
		}
	}

	Ok(())
}

fn collect_columns(stmt: *mut libsqlite3_sys::sqlite3_stmt) -> Vec<String> {
	let column_count = unsafe { sqlite3_column_count(stmt) };
	(0..column_count)
		.map(|index| unsafe {
			let name_ptr = sqlite3_column_name(stmt, index);
			if name_ptr.is_null() {
				String::new()
			} else {
				CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
			}
		})
		.collect()
}

fn column_value(stmt: *mut libsqlite3_sys::sqlite3_stmt, index: i32) -> ColumnValue {
	match unsafe { sqlite3_column_type(stmt, index) } {
		SQLITE_NULL => ColumnValue::Null,
		SQLITE_INTEGER => ColumnValue::Integer(unsafe { sqlite3_column_int64(stmt, index) }),
		SQLITE_FLOAT => ColumnValue::Float(unsafe { sqlite3_column_double(stmt, index) }),
		SQLITE_TEXT => {
			let text_ptr = unsafe { sqlite3_column_text(stmt, index) };
			if text_ptr.is_null() {
				ColumnValue::Null
			} else {
				let text = unsafe { CStr::from_ptr(text_ptr as *const c_char) }
					.to_string_lossy()
					.into_owned();
				ColumnValue::Text(text)
			}
		}
		SQLITE_BLOB => {
			let blob_ptr = unsafe { sqlite3_column_blob(stmt, index) };
			if blob_ptr.is_null() {
				ColumnValue::Null
			} else {
				let blob_len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
				let blob = unsafe { std::slice::from_raw_parts(blob_ptr as *const u8, blob_len) };
				ColumnValue::Blob(blob.to_vec())
			}
		}
		_ => ColumnValue::Null,
	}
}

fn sqlite_error(db: *mut sqlite3, context: &str) -> anyhow::Error {
	let message = unsafe {
		if db.is_null() {
			"unknown sqlite error".to_string()
		} else {
			CStr::from_ptr(sqlite3_errmsg(db))
				.to_string_lossy()
				.into_owned()
		}
	};
	anyhow!("{context}: {message}")
}

#[cfg(test)]
mod tests {
	use super::*;
	use libsqlite3_sys::{sqlite3_close, sqlite3_open};

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
	fn run_and_query_bind_typed_params() {
		let db = MemoryDb::open();
		exec_statements(
			db.as_ptr(),
			"CREATE TABLE items(id INTEGER PRIMARY KEY, label TEXT, score REAL, payload BLOB);",
		)
		.unwrap();

		let result = execute_statement(
			db.as_ptr(),
			"INSERT INTO items(label, score, payload) VALUES (?, ?, ?);",
			Some(&[
				BindParam::Text("alpha".to_owned()),
				BindParam::Float(3.5),
				BindParam::Blob(vec![1, 2, 3]),
			]),
		)
		.unwrap();
		assert_eq!(result.changes, 1);

		let rows = query_statement(
			db.as_ptr(),
			"SELECT id, label, score, payload FROM items WHERE label = ?;",
			Some(&[BindParam::Text("alpha".to_owned())]),
		)
		.unwrap();
		assert_eq!(rows.columns, vec!["id", "label", "score", "payload"]);
		assert_eq!(
			rows.rows,
			vec![vec![
				ColumnValue::Integer(1),
				ColumnValue::Text("alpha".to_owned()),
				ColumnValue::Float(3.5),
				ColumnValue::Blob(vec![1, 2, 3]),
			]]
		);
	}

	#[test]
	fn exec_returns_last_statement_rows() {
		let db = MemoryDb::open();
		let result = exec_statements(
			db.as_ptr(),
			"CREATE TABLE items(id INTEGER); INSERT INTO items VALUES (1), (2); SELECT COUNT(*) AS count FROM items;",
		)
		.unwrap();

		assert_eq!(result.columns, vec!["count"]);
		assert_eq!(result.rows, vec![vec![ColumnValue::Integer(2)]]);
	}
}
