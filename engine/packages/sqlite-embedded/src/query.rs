use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::{c_char, c_int};
use std::ptr;

use anyhow::{Result, anyhow};
use libsqlite3_sys::{
	SQLITE_BLOB, SQLITE_DONE, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW,
	SQLITE_TEXT, SQLITE_TRANSIENT, sqlite3, sqlite3_bind_blob, sqlite3_bind_double,
	sqlite3_bind_int64, sqlite3_bind_null, sqlite3_bind_text, sqlite3_changes, sqlite3_column_blob,
	sqlite3_column_bytes, sqlite3_column_count, sqlite3_column_double, sqlite3_column_int64,
	sqlite3_column_name, sqlite3_column_text, sqlite3_column_type, sqlite3_errmsg,
	sqlite3_extended_errcode, sqlite3_file_control, sqlite3_finalize, sqlite3_last_insert_rowid,
	sqlite3_prepare_v2, sqlite3_sql, sqlite3_step, sqlite3_stmt,
};
pub use sqlite_types::{BindParam, ColumnValue, ExecResult, ExecuteResult, QueryResult};

use crate::vfs::{FCNTL_COMMIT_PAGE_CAP, SqliteTransactionTooLargeError};

#[derive(Clone, Copy, Debug, Default)]
pub struct SqliteQueryProfile {
	pub bind_count: u64,
	pub bind_logical_bytes: u64,
	pub result_rows: u64,
	pub result_columns: u64,
	pub result_logical_bytes: u64,
}

#[derive(Debug)]
pub struct SqliteStatementError {
	/// SQLite's extended result code, or -1 when the error did not come from SQLite.
	pub code: i32,
	pub statement_index: u32,
	pub message: String,
	/// SQLite's own message, without the operation that failed.
	pub detail: String,
	/// Byte offset into the statement's SQL of the token SQLite blamed, when it named one.
	pub offset: Option<u32>,
}

impl fmt::Display for SqliteStatementError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.message)
	}
}

impl Error for SqliteStatementError {}

/// Logical bytes every result value costs before its text or blob bytes.
const RESULT_VALUE_BYTES: u64 = 8;

/// A statement's result grew past the command's result byte cap. The statement is stopped
/// before the value that crossed the cap is copied out of SQLite.
#[derive(Debug)]
pub struct SqliteResultTooLargeError {
	pub max_bytes: u64,
}

impl fmt::Display for SqliteResultTooLargeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"sqlite response too large: the result passes the {} byte limit",
			self.max_bytes
		)
	}
}

impl Error for SqliteResultTooLargeError {}

/// Charges one statement's result values against an optional logical byte cap while its rows
/// are produced. Each value costs 8 bytes plus its text or blob length, so the result is
/// refused before its bytes are copied rather than after the whole result is built.
struct ResultBudget {
	max_bytes: Option<u64>,
	used_bytes: u64,
}

impl ResultBudget {
	fn new(max_bytes: Option<u64>) -> Self {
		Self {
			max_bytes,
			used_bytes: 0,
		}
	}

	/// Charges the value in column `index` of the current row and returns the value bytes it
	/// charged. Call it before the value is read with `sqlite3_column_text` or
	/// `sqlite3_column_blob`.
	fn charge(&mut self, stmt: *mut sqlite3_stmt, index: i32) -> Result<u64> {
		if self.max_bytes.is_none() {
			return Ok(0);
		}

		let value_bytes = match unsafe { sqlite3_column_type(stmt, index) } {
			SQLITE_TEXT | SQLITE_BLOB => unsafe { sqlite3_column_bytes(stmt, index) }.max(0) as u64,
			_ => 0,
		};
		self.add(RESULT_VALUE_BYTES.saturating_add(value_bytes))?;
		Ok(value_bytes)
	}

	/// Charges the bytes a text value grew by when invalid UTF-8 was replaced while copying
	/// it, so the cap bounds the copied result and not only SQLite's bytes.
	fn charge_conversion(&mut self, charged_bytes: u64, value: &ColumnValue) -> Result<()> {
		if let ColumnValue::Text(text) = value {
			let text_bytes = text.len() as u64;
			if text_bytes > charged_bytes {
				self.add(text_bytes - charged_bytes)?;
			}
		}
		Ok(())
	}

	fn add(&mut self, bytes: u64) -> Result<()> {
		let Some(max_bytes) = self.max_bytes else {
			return Ok(());
		};

		self.used_bytes = self.used_bytes.saturating_add(bytes);
		if self.used_bytes > max_bytes {
			return Err(SqliteResultTooLargeError { max_bytes }.into());
		}

		Ok(())
	}
}

/// Reads the current row, charging every value to `budget` before copying it.
fn read_row(
	stmt: *mut sqlite3_stmt,
	column_count: usize,
	budget: &mut ResultBudget,
) -> Result<Vec<ColumnValue>> {
	let mut row = Vec::with_capacity(column_count);
	for index in 0..column_count {
		let charged_bytes = budget.charge(stmt, index as i32)?;
		let value = column_value(stmt, index as i32);
		budget.charge_conversion(charged_bytes, &value)?;
		row.push(value);
	}
	Ok(row)
}

/// Like [`read_row`], also counting the row into `profile`.
fn read_row_profiled(
	stmt: *mut sqlite3_stmt,
	column_count: usize,
	budget: &mut ResultBudget,
	profile: &mut SqliteQueryProfile,
) -> Result<Vec<ColumnValue>> {
	let mut row = Vec::with_capacity(column_count);
	for index in 0..column_count {
		let charged_bytes = budget.charge(stmt, index as i32)?;
		let value = column_value_profiled(stmt, index as i32, profile);
		budget.charge_conversion(charged_bytes, &value)?;
		row.push(value);
	}
	Ok(row)
}

pub fn execute_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[BindParam]>,
) -> Result<ExecResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	let _stmt = FinalizeOnDrop(stmt);
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, 0, "failed to prepare sqlite statement"));
	}
	if stmt.is_null() {
		return Ok(ExecResult { changes: 0 });
	}

	refuse_oversize_vacuum(db, stmt)?;
	if let Some(params) = params {
		bind_params(db, stmt, params)?;
	}

	loop {
		let step_rc = unsafe { sqlite3_step(stmt) };
		if step_rc == SQLITE_DONE {
			break;
		}
		if step_rc != SQLITE_ROW {
			return Err(sqlite_error(db, 0, "failed to execute sqlite statement"));
		}
	}

	Ok(ExecResult {
		changes: unsafe { sqlite3_changes(db) as i64 },
	})
}

pub fn query_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[BindParam]>,
) -> Result<QueryResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	let _stmt = FinalizeOnDrop(stmt);
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, 0, "failed to prepare sqlite query"));
	}
	if stmt.is_null() {
		return Ok(QueryResult {
			columns: Vec::new(),
			rows: Vec::new(),
		});
	}

	refuse_oversize_vacuum(db, stmt)?;
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
			return Err(sqlite_error(db, 0, "failed to step sqlite query"));
		}

		let mut row = Vec::with_capacity(columns.len());
		for index in 0..columns.len() {
			row.push(column_value(stmt, index as i32));
		}
		rows.push(row);
	}

	Ok(QueryResult { columns, rows })
}

/// Runs one statement and collects its rows. With `max_result_bytes`, the statement stops with
/// [`SqliteResultTooLargeError`] as soon as its result passes that many logical bytes.
pub fn execute_single_statement(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[BindParam]>,
	max_result_bytes: Option<u64>,
) -> Result<ExecuteResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let mut tail = ptr::null();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, &mut tail) };
	let _stmt = FinalizeOnDrop(stmt);
	if rc != SQLITE_OK {
		return Err(sqlite_error(
			db,
			0,
			"failed to prepare sqlite execute statement",
		));
	}
	if has_non_whitespace_tail(tail) {
		return Err(SqliteStatementError {
			code: -1,
			statement_index: 0,
			message: "sqlite execute only supports a single statement".to_owned(),
			detail: "sqlite execute only supports a single statement".to_owned(),
			offset: None,
		}
		.into());
	}
	if stmt.is_null() {
		return Ok(ExecuteResult {
			columns: Vec::new(),
			rows: Vec::new(),
			changes: 0,
			last_insert_row_id: None,
		});
	}

	refuse_oversize_vacuum(db, stmt)?;
	if let Some(params) = params {
		bind_params(db, stmt, params)?;
	}

	let columns = collect_columns(stmt);
	let mut rows = Vec::new();
	let mut budget = ResultBudget::new(max_result_bytes);
	loop {
		let step_rc = unsafe { sqlite3_step(stmt) };
		if step_rc == SQLITE_DONE {
			break;
		}
		if step_rc != SQLITE_ROW {
			return Err(sqlite_error(
				db,
				0,
				"failed to step sqlite execute statement",
			));
		}

		rows.push(read_row(stmt, columns.len(), &mut budget)?);
	}

	let changes = unsafe { sqlite3_changes(db) as i64 };
	Ok(ExecuteResult {
		columns,
		rows,
		changes,
		last_insert_row_id: (changes > 0).then(|| unsafe { sqlite3_last_insert_rowid(db) }),
	})
}

/// Like [`execute_single_statement`], also returning the statement's profile.
pub fn execute_single_statement_profiled(
	db: *mut sqlite3,
	sql: &str,
	params: Option<&[BindParam]>,
	max_result_bytes: Option<u64>,
) -> Result<(ExecuteResult, SqliteQueryProfile)> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut stmt = ptr::null_mut();
	let mut tail = ptr::null();
	let rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, &mut tail) };
	let _stmt = FinalizeOnDrop(stmt);
	if rc != SQLITE_OK {
		return Err(sqlite_error(
			db,
			0,
			"failed to prepare sqlite execute statement",
		));
	}
	if has_non_whitespace_tail(tail) {
		return Err(SqliteStatementError {
			code: -1,
			statement_index: 0,
			message: "sqlite execute only supports a single statement".to_owned(),
			detail: "sqlite execute only supports a single statement".to_owned(),
			offset: None,
		}
		.into());
	}
	if stmt.is_null() {
		return Ok((
			ExecuteResult {
				columns: Vec::new(),
				rows: Vec::new(),
				changes: 0,
				last_insert_row_id: None,
			},
			SqliteQueryProfile::default(),
		));
	}

	refuse_oversize_vacuum(db, stmt)?;
	let mut profile = SqliteQueryProfile::default();
	if let Some(params) = params {
		bind_params_profiled(db, stmt, params, &mut profile)?;
	}

	let columns = collect_columns(stmt);
	profile.result_columns = columns.len() as u64;
	let mut rows = Vec::new();
	let mut budget = ResultBudget::new(max_result_bytes);
	loop {
		let step_rc = unsafe { sqlite3_step(stmt) };
		if step_rc == SQLITE_DONE {
			break;
		}
		if step_rc != SQLITE_ROW {
			return Err(sqlite_error(
				db,
				0,
				"failed to step sqlite execute statement",
			));
		}

		rows.push(read_row_profiled(
			stmt,
			columns.len(),
			&mut budget,
			&mut profile,
		)?);
		profile.result_rows = profile.result_rows.saturating_add(1);
	}

	let changes = unsafe { sqlite3_changes(db) as i64 };
	Ok((
		ExecuteResult {
			columns,
			rows,
			changes,
			last_insert_row_id: (changes > 0).then(|| unsafe { sqlite3_last_insert_rowid(db) }),
		},
		profile,
	))
}

/// Runs every statement in `sql` and returns the rows of the last one that has columns or
/// rows. With `max_result_bytes`, each statement's result is capped at that many logical bytes.
pub fn exec_statements(
	db: *mut sqlite3,
	sql: &str,
	max_result_bytes: Option<u64>,
) -> Result<QueryResult> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut remaining = c_sql.as_ptr();
	let mut statement_index = 0_u32;
	let mut final_result = QueryResult {
		columns: Vec::new(),
		rows: Vec::new(),
	};

	while unsafe { *remaining } != 0 {
		let mut stmt = ptr::null_mut();
		let mut tail = ptr::null();
		let rc = unsafe { sqlite3_prepare_v2(db, remaining, -1, &mut stmt, &mut tail) };
		let _stmt = FinalizeOnDrop(stmt);
		if rc != SQLITE_OK {
			return Err(sqlite_error(
				db,
				statement_index,
				"failed to prepare sqlite exec statement",
			));
		}

		if stmt.is_null() {
			if tail == remaining {
				break;
			}
			remaining = tail;
			continue;
		}

		// A statement with columns replaces the earlier result, so release that result before
		// this statement's rows are collected and only one result is held at a time.
		if unsafe { sqlite3_column_count(stmt) } > 0 {
			final_result = QueryResult {
				columns: Vec::new(),
				rows: Vec::new(),
			};
		}

		refuse_oversize_vacuum(db, stmt)?;
		let columns = collect_columns(stmt);
		let mut rows = Vec::new();
		let mut budget = ResultBudget::new(max_result_bytes);
		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				return Err(sqlite_error(
					db,
					statement_index,
					"failed to step sqlite exec statement",
				));
			}

			rows.push(read_row(stmt, columns.len(), &mut budget)?);
		}

		if !columns.is_empty() || !rows.is_empty() {
			final_result = QueryResult { columns, rows };
		}

		if tail == remaining {
			break;
		}
		remaining = tail;
		statement_index = statement_index.saturating_add(1);
	}

	Ok(final_result)
}

/// Like [`exec_statements`], also returning the profile of the returned statement.
pub fn exec_statements_profiled(
	db: *mut sqlite3,
	sql: &str,
	max_result_bytes: Option<u64>,
) -> Result<(QueryResult, SqliteQueryProfile)> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut remaining = c_sql.as_ptr();
	let mut statement_index = 0_u32;
	let mut final_result = QueryResult {
		columns: Vec::new(),
		rows: Vec::new(),
	};
	let mut final_profile = SqliteQueryProfile::default();

	while unsafe { *remaining } != 0 {
		let mut stmt = ptr::null_mut();
		let mut tail = ptr::null();
		let rc = unsafe { sqlite3_prepare_v2(db, remaining, -1, &mut stmt, &mut tail) };
		let _stmt = FinalizeOnDrop(stmt);
		if rc != SQLITE_OK {
			return Err(sqlite_error(
				db,
				statement_index,
				"failed to prepare sqlite exec statement",
			));
		}

		if stmt.is_null() {
			if tail == remaining {
				break;
			}
			remaining = tail;
			continue;
		}

		// A statement with columns replaces the earlier result, so release that result before
		// this statement's rows are collected and only one result is held at a time.
		if unsafe { sqlite3_column_count(stmt) } > 0 {
			final_result = QueryResult {
				columns: Vec::new(),
				rows: Vec::new(),
			};
		}

		refuse_oversize_vacuum(db, stmt)?;
		let columns = collect_columns(stmt);
		let mut profile = SqliteQueryProfile {
			result_columns: columns.len() as u64,
			..Default::default()
		};
		let mut rows = Vec::new();
		let mut budget = ResultBudget::new(max_result_bytes);
		loop {
			let step_rc = unsafe { sqlite3_step(stmt) };
			if step_rc == SQLITE_DONE {
				break;
			}
			if step_rc != SQLITE_ROW {
				return Err(sqlite_error(
					db,
					statement_index,
					"failed to step sqlite exec statement",
				));
			}

			rows.push(read_row_profiled(
				stmt,
				columns.len(),
				&mut budget,
				&mut profile,
			)?);
			profile.result_rows = profile.result_rows.saturating_add(1);
		}

		if !columns.is_empty() || !rows.is_empty() {
			final_result = QueryResult { columns, rows };
			final_profile = profile;
		}

		if tail == remaining {
			break;
		}
		remaining = tail;
		statement_index = statement_index.saturating_add(1);
	}

	Ok((final_result, final_profile))
}

fn bind_params(
	db: *mut sqlite3,
	stmt: *mut libsqlite3_sys::sqlite3_stmt,
	params: &[BindParam],
) -> Result<()> {
	for (index, param) in params.iter().enumerate() {
		bind_param(db, stmt, (index + 1) as i32, param)?;
	}

	Ok(())
}

fn bind_params_profiled(
	db: *mut sqlite3,
	stmt: *mut libsqlite3_sys::sqlite3_stmt,
	params: &[BindParam],
	profile: &mut SqliteQueryProfile,
) -> Result<()> {
	for (index, param) in params.iter().enumerate() {
		profile.bind_count = profile.bind_count.saturating_add(1);
		profile.bind_logical_bytes = profile
			.bind_logical_bytes
			.saturating_add(logical_bind_bytes(param));
		bind_param(db, stmt, (index + 1) as i32, param)?;
	}

	Ok(())
}

fn bind_param(
	db: *mut sqlite3,
	stmt: *mut libsqlite3_sys::sqlite3_stmt,
	bind_index: i32,
	param: &BindParam,
) -> Result<()> {
	let rc = match param {
		BindParam::Null => unsafe { sqlite3_bind_null(stmt, bind_index) },
		BindParam::Integer(value) => unsafe { sqlite3_bind_int64(stmt, bind_index, *value) },
		BindParam::Float(value) => unsafe { sqlite3_bind_double(stmt, bind_index, *value) },
		BindParam::Text(value) => unsafe {
			sqlite3_bind_text(
				stmt,
				bind_index,
				value.as_ptr() as *const c_char,
				value.len() as i32,
				SQLITE_TRANSIENT(),
			)
		},
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
		return Err(sqlite_error(db, 0, "failed to bind sqlite parameter"));
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
				let text_len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
				let text = String::from_utf8_lossy(unsafe {
					std::slice::from_raw_parts(text_ptr as *const u8, text_len)
				})
				.into_owned();
				ColumnValue::Text(text)
			}
		}
		SQLITE_BLOB => {
			let blob_ptr = unsafe { sqlite3_column_blob(stmt, index) };
			let blob_len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
			// SQLite returns a null pointer for an empty BLOB, distinct from SQL NULL.
			// Do not construct a slice from that pointer, even with length zero.
			if blob_len == 0 {
				ColumnValue::Blob(Vec::new())
			} else if blob_ptr.is_null() {
				ColumnValue::Null
			} else {
				let blob = unsafe { std::slice::from_raw_parts(blob_ptr as *const u8, blob_len) };
				ColumnValue::Blob(blob.to_vec())
			}
		}
		_ => ColumnValue::Null,
	}
}

fn column_value_profiled(
	stmt: *mut libsqlite3_sys::sqlite3_stmt,
	index: i32,
	profile: &mut SqliteQueryProfile,
) -> ColumnValue {
	let mut logical_bytes = 0_u64;
	let value = match unsafe { sqlite3_column_type(stmt, index) } {
		SQLITE_NULL => ColumnValue::Null,
		SQLITE_INTEGER => {
			logical_bytes = 8;
			ColumnValue::Integer(unsafe { sqlite3_column_int64(stmt, index) })
		}
		SQLITE_FLOAT => {
			logical_bytes = 8;
			ColumnValue::Float(unsafe { sqlite3_column_double(stmt, index) })
		}
		SQLITE_TEXT => {
			let text_ptr = unsafe { sqlite3_column_text(stmt, index) };
			if text_ptr.is_null() {
				ColumnValue::Null
			} else {
				let text_len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
				logical_bytes = text_len as u64;
				let text = String::from_utf8_lossy(unsafe {
					std::slice::from_raw_parts(text_ptr as *const u8, text_len)
				})
				.into_owned();
				ColumnValue::Text(text)
			}
		}
		SQLITE_BLOB => {
			let blob_ptr = unsafe { sqlite3_column_blob(stmt, index) };
			let blob_len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
			// SQLite returns a null pointer for an empty BLOB, distinct from SQL NULL.
			// Do not construct a slice from that pointer, even with length zero.
			if blob_len == 0 {
				ColumnValue::Blob(Vec::new())
			} else if blob_ptr.is_null() {
				ColumnValue::Null
			} else {
				logical_bytes = blob_len as u64;
				let blob = unsafe { std::slice::from_raw_parts(blob_ptr as *const u8, blob_len) };
				ColumnValue::Blob(blob.to_vec())
			}
		}
		_ => ColumnValue::Null,
	};
	profile.result_logical_bytes = profile.result_logical_bytes.saturating_add(logical_bytes);
	value
}

fn logical_bind_bytes(param: &BindParam) -> u64 {
	match param {
		BindParam::Null => 0,
		BindParam::Integer(_) | BindParam::Float(_) => 8,
		BindParam::Text(value) => value.len() as u64,
		BindParam::Blob(value) => value.len() as u64,
	}
}

/// Refuses an in-place `VACUUM` before it runs when the database holds more in-use pages than
/// one commit may carry.
///
/// VACUUM rewrites the whole database in one transaction. Without this check it reads every
/// page and builds the full copy in memory before the VFS can refuse the copy itself. In-use
/// pages are an estimate of the copy's size: a sparsely filled database can compact below the
/// cap and is still refused here, which keeps a large VACUUM from holding a whole copy in
/// memory. Connections on other VFSes report no cap and are not checked.
fn refuse_oversize_vacuum(db: *mut sqlite3, stmt: *mut sqlite3_stmt) -> Result<()> {
	let sql = unsafe { sqlite3_sql(stmt) };
	if sql.is_null() {
		return Ok(());
	}
	let sql = unsafe { CStr::from_ptr(sql) }.to_string_lossy();
	if !is_in_place_vacuum(&sql) {
		return Ok(());
	}

	let mut max_pages: i64 = 0;
	let rc = unsafe {
		sqlite3_file_control(
			db,
			c"main".as_ptr(),
			FCNTL_COMMIT_PAGE_CAP,
			(&mut max_pages as *mut i64).cast(),
		)
	};
	if rc != SQLITE_OK {
		return Ok(());
	}

	let (Some(page_count), Some(freelist_count), Some(page_size)) = (
		pragma_i64(db, c"PRAGMA main.page_count;"),
		pragma_i64(db, c"PRAGMA main.freelist_count;"),
		pragma_i64(db, c"PRAGMA main.page_size;"),
	) else {
		return Ok(());
	};
	let in_use_pages = page_count.saturating_sub(freelist_count);
	if in_use_pages > max_pages {
		return Err(SqliteTransactionTooLargeError {
			max_pages: max_pages as usize,
			page_size: page_size as usize,
		}
		.into());
	}

	Ok(())
}

/// Whether a statement is a `VACUUM` of the main database in place. `VACUUM INTO` writes a
/// separate file and `VACUUM temp` another schema, so neither commits to the main database.
pub(crate) fn is_in_place_vacuum(sql: &str) -> bool {
	let words = sql_words(sql);
	let mut words = words.iter();
	if !words
		.next()
		.is_some_and(|word| word.eq_ignore_ascii_case("vacuum"))
	{
		return false;
	}

	match words.next() {
		None => true,
		Some(word) if word.eq_ignore_ascii_case("into") => false,
		Some(schema) => {
			let schema = schema.trim_matches(|c| matches!(c, '"' | '`' | '[' | ']' | '\''));
			schema.eq_ignore_ascii_case("main")
				&& !words.any(|word| word.eq_ignore_ascii_case("into"))
		}
	}
}

/// Splits SQL into whitespace or `;` separated words, dropping `--` and `/* */` comments.
/// Quoted text is not interpreted, which is enough to read a statement's leading keywords.
fn sql_words(sql: &str) -> Vec<&str> {
	let mut words = Vec::new();
	let mut rest = sql;
	loop {
		rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ';');
		if rest.is_empty() {
			return words;
		}
		if let Some(comment) = rest.strip_prefix("--") {
			rest = comment.find('\n').map_or("", |end| &comment[end + 1..]);
			continue;
		}
		if let Some(comment) = rest.strip_prefix("/*") {
			rest = comment.find("*/").map_or("", |end| &comment[end + 2..]);
			continue;
		}
		let end = rest
			.char_indices()
			.find(|(index, c)| {
				c.is_ascii_whitespace()
					|| *c == ';' || rest[*index..].starts_with("--")
					|| rest[*index..].starts_with("/*")
			})
			.map_or(rest.len(), |(index, _)| index);
		words.push(&rest[..end]);
		rest = &rest[end..];
	}
}

/// Reads a single integer pragma, or `None` if it fails.
fn pragma_i64(db: *mut sqlite3, sql: &CStr) -> Option<i64> {
	let mut stmt = ptr::null_mut();
	let rc = unsafe { sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
	let _stmt = FinalizeOnDrop(stmt);
	if rc != SQLITE_OK || stmt.is_null() {
		return None;
	}
	if unsafe { sqlite3_step(stmt) } == SQLITE_ROW {
		Some(unsafe { sqlite3_column_int64(stmt, 0) })
	} else {
		None
	}
}

/// Finalizes a prepared statement when its owner leaves scope, including by an error return or
/// a panic, so no statement outlives the call that prepared it.
struct FinalizeOnDrop(*mut sqlite3_stmt);

impl Drop for FinalizeOnDrop {
	fn drop(&mut self) {
		// SAFETY: the statement was prepared by the call that owns this guard and is used by no
		// one else. Finalizing a null statement is a no-op.
		unsafe {
			sqlite3_finalize(self.0);
		}
	}
}

fn has_non_whitespace_tail(tail: *const c_char) -> bool {
	if tail.is_null() {
		return false;
	}

	let bytes = unsafe { CStr::from_ptr(tail).to_bytes() };
	bytes.iter().any(|byte| !byte.is_ascii_whitespace())
}

// The bundled SQLite has `sqlite3_error_offset`, but libsqlite3-sys's default bindings target an
// older SQLite that predates it.
unsafe extern "C" {
	fn sqlite3_error_offset(db: *mut sqlite3) -> c_int;
}

fn sqlite_error(db: *mut sqlite3, statement_index: u32, context: &str) -> anyhow::Error {
	let (code, detail, offset) = unsafe {
		if db.is_null() {
			(-1, "unknown sqlite error".to_string(), -1)
		} else {
			(
				sqlite3_extended_errcode(db),
				CStr::from_ptr(sqlite3_errmsg(db))
					.to_string_lossy()
					.into_owned(),
				sqlite3_error_offset(db),
			)
		}
	};
	SqliteStatementError {
		code,
		statement_index,
		message: format!("{context}: {detail}"),
		detail,
		offset: u32::try_from(offset).ok(),
	}
	.into()
}


