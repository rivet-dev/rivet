use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

use anyhow::{Result, anyhow};
use libsqlite3_sys::{
	SQLITE_ALTER_TABLE, SQLITE_ANALYZE, SQLITE_ATTACH, SQLITE_BLOB, SQLITE_CREATE_INDEX,
	SQLITE_CREATE_TABLE, SQLITE_CREATE_TEMP_INDEX, SQLITE_CREATE_TEMP_TABLE,
	SQLITE_CREATE_TEMP_TRIGGER, SQLITE_CREATE_TEMP_VIEW, SQLITE_CREATE_TRIGGER,
	SQLITE_CREATE_VIEW, SQLITE_CREATE_VTABLE, SQLITE_DELETE, SQLITE_DENY, SQLITE_DETACH,
	SQLITE_DONE, SQLITE_DROP_INDEX, SQLITE_DROP_TABLE, SQLITE_DROP_TEMP_INDEX,
	SQLITE_DROP_TEMP_TABLE, SQLITE_DROP_TEMP_TRIGGER, SQLITE_DROP_TEMP_VIEW,
	SQLITE_DROP_TRIGGER, SQLITE_DROP_VIEW, SQLITE_DROP_VTABLE, SQLITE_FLOAT, SQLITE_FUNCTION,
	SQLITE_INSERT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_PRAGMA, SQLITE_READ,
	SQLITE_REINDEX, SQLITE_ROW, SQLITE_SAVEPOINT, SQLITE_SELECT, SQLITE_TEXT,
	SQLITE_TRANSACTION, SQLITE_TRANSIENT, SQLITE_UPDATE, sqlite3, sqlite3_bind_blob,
	sqlite3_bind_double, sqlite3_bind_int64, sqlite3_bind_null, sqlite3_bind_text,
	sqlite3_changes, sqlite3_column_blob, sqlite3_column_bytes, sqlite3_column_count,
	sqlite3_column_double, sqlite3_column_int64, sqlite3_column_name, sqlite3_column_text,
	sqlite3_column_type, sqlite3_errmsg, sqlite3_finalize, sqlite3_prepare_v2,
	sqlite3_set_authorizer, sqlite3_step, sqlite3_stmt_readonly,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementClassification {
	pub has_statement: bool,
	pub sqlite_readonly: bool,
	pub has_trailing_sql: bool,
	pub authorizer: StatementAuthorizerSummary,
}

impl StatementClassification {
	pub fn reader_eligible(&self) -> bool {
		self.has_statement
			&& self.sqlite_readonly
			&& !self.has_trailing_sql
			&& !self.authorizer.requires_write_route()
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatementAuthorizerSummary {
	pub transaction_control: bool,
	pub attach: bool,
	pub detach: bool,
	pub schema_writes: bool,
	pub temp_writes: bool,
	pub pragma_usage: bool,
	pub function_calls: bool,
	pub write_operations: bool,
	pub actions: Vec<StatementAuthorizerAction>,
}

impl StatementAuthorizerSummary {
	pub fn requires_write_route(&self) -> bool {
		self.transaction_control
			|| self.attach
			|| self.detach
			|| self.schema_writes
			|| self.temp_writes
			|| self.write_operations
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementAuthorizerAction {
	pub kind: StatementAuthorizerActionKind,
	pub first_arg: Option<String>,
	pub second_arg: Option<String>,
	pub database_name: Option<String>,
	pub trigger_or_view_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementAuthorizerActionKind {
	Read,
	Select,
	Transaction,
	Savepoint,
	Attach,
	Detach,
	Pragma,
	Function,
	Insert,
	Update,
	Delete,
	CreateIndex,
	CreateTable,
	CreateTrigger,
	CreateView,
	CreateVirtualTable,
	CreateTempIndex,
	CreateTempTable,
	CreateTempTrigger,
	CreateTempView,
	DropIndex,
	DropTable,
	DropTrigger,
	DropView,
	DropVirtualTable,
	DropTempIndex,
	DropTempTable,
	DropTempTrigger,
	DropTempView,
	AlterTable,
	Reindex,
	Analyze,
	Other(i32),
}

impl StatementAuthorizerActionKind {
	fn from_code(code: c_int) -> Self {
		match code {
			SQLITE_READ => Self::Read,
			SQLITE_SELECT => Self::Select,
			SQLITE_TRANSACTION => Self::Transaction,
			SQLITE_SAVEPOINT => Self::Savepoint,
			SQLITE_ATTACH => Self::Attach,
			SQLITE_DETACH => Self::Detach,
			SQLITE_PRAGMA => Self::Pragma,
			SQLITE_FUNCTION => Self::Function,
			SQLITE_INSERT => Self::Insert,
			SQLITE_UPDATE => Self::Update,
			SQLITE_DELETE => Self::Delete,
			SQLITE_CREATE_INDEX => Self::CreateIndex,
			SQLITE_CREATE_TABLE => Self::CreateTable,
			SQLITE_CREATE_TRIGGER => Self::CreateTrigger,
			SQLITE_CREATE_VIEW => Self::CreateView,
			SQLITE_CREATE_VTABLE => Self::CreateVirtualTable,
			SQLITE_CREATE_TEMP_INDEX => Self::CreateTempIndex,
			SQLITE_CREATE_TEMP_TABLE => Self::CreateTempTable,
			SQLITE_CREATE_TEMP_TRIGGER => Self::CreateTempTrigger,
			SQLITE_CREATE_TEMP_VIEW => Self::CreateTempView,
			SQLITE_DROP_INDEX => Self::DropIndex,
			SQLITE_DROP_TABLE => Self::DropTable,
			SQLITE_DROP_TRIGGER => Self::DropTrigger,
			SQLITE_DROP_VIEW => Self::DropView,
			SQLITE_DROP_VTABLE => Self::DropVirtualTable,
			SQLITE_DROP_TEMP_INDEX => Self::DropTempIndex,
			SQLITE_DROP_TEMP_TABLE => Self::DropTempTable,
			SQLITE_DROP_TEMP_TRIGGER => Self::DropTempTrigger,
			SQLITE_DROP_TEMP_VIEW => Self::DropTempView,
			SQLITE_ALTER_TABLE => Self::AlterTable,
			SQLITE_REINDEX => Self::Reindex,
			SQLITE_ANALYZE => Self::Analyze,
			_ => Self::Other(code),
		}
	}

	fn is_schema_write(&self) -> bool {
		matches!(
			self,
			Self::CreateIndex
				| Self::CreateTable
				| Self::CreateTrigger
				| Self::CreateView
				| Self::CreateVirtualTable
				| Self::DropIndex
				| Self::DropTable
				| Self::DropTrigger
				| Self::DropView
				| Self::DropVirtualTable
				| Self::AlterTable
				| Self::Reindex
				| Self::Analyze
		)
	}

	fn is_temp_schema_write(&self) -> bool {
		matches!(
			self,
			Self::CreateTempIndex
				| Self::CreateTempTable
				| Self::CreateTempTrigger
				| Self::CreateTempView
				| Self::DropTempIndex
				| Self::DropTempTable
				| Self::DropTempTrigger
				| Self::DropTempView
		)
	}

	fn is_data_write(&self) -> bool {
		matches!(self, Self::Insert | Self::Update | Self::Delete)
	}
}

pub fn classify_statement(db: *mut sqlite3, sql: &str) -> Result<StatementClassification> {
	let c_sql = CString::new(sql).map_err(|err| anyhow!(err.to_string()))?;
	let mut summary = StatementAuthorizerSummary::default();
	let rc = unsafe {
		sqlite3_set_authorizer(
			db,
			Some(capture_authorizer_action),
			&mut summary as *mut StatementAuthorizerSummary as *mut c_void,
		)
	};
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, "failed to install sqlite authorizer"));
	}

	let mut stmt = ptr::null_mut();
	let mut tail = ptr::null();
	let prepare_rc = unsafe { sqlite3_prepare_v2(db, c_sql.as_ptr(), -1, &mut stmt, &mut tail) };
	let prepare_error = if prepare_rc == SQLITE_OK {
		None
	} else {
		Some(sqlite_error(db, "failed to prepare sqlite statement for classification"))
	};

	let restore_rc = unsafe { sqlite3_set_authorizer(db, None, ptr::null_mut()) };
	if restore_rc != SQLITE_OK {
		if !stmt.is_null() {
			unsafe {
				sqlite3_finalize(stmt);
			}
		}
		return Err(sqlite_error(db, "failed to clear sqlite authorizer"));
	}

	if let Some(err) = prepare_error {
		if !stmt.is_null() {
			unsafe {
				sqlite3_finalize(stmt);
			}
		}
		return Err(err);
	}

	if stmt.is_null() {
		return Ok(StatementClassification {
			has_statement: false,
			sqlite_readonly: true,
			has_trailing_sql: has_non_whitespace_tail(tail),
			authorizer: summary,
		});
	}

	let sqlite_readonly = unsafe { sqlite3_stmt_readonly(stmt) != 0 };
	unsafe {
		sqlite3_finalize(stmt);
	}

	Ok(StatementClassification {
		has_statement: true,
		sqlite_readonly,
		has_trailing_sql: has_non_whitespace_tail(tail),
		authorizer: summary,
	})
}

pub fn install_reader_authorizer(db: *mut sqlite3) -> Result<()> {
	let rc = unsafe {
		sqlite3_set_authorizer(db, Some(reader_authorizer_action), ptr::null_mut())
	};
	if rc != SQLITE_OK {
		return Err(sqlite_error(db, "failed to install sqlite reader authorizer"));
	}

	Ok(())
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

unsafe extern "C" fn capture_authorizer_action(
	user_data: *mut c_void,
	action_code: c_int,
	first_arg: *const c_char,
	second_arg: *const c_char,
	database_name: *const c_char,
	trigger_or_view_name: *const c_char,
) -> c_int {
	if user_data.is_null() {
		return SQLITE_OK;
	}

	let summary = unsafe { &mut *(user_data as *mut StatementAuthorizerSummary) };
	let kind = StatementAuthorizerActionKind::from_code(action_code);
	let database_name = unsafe { optional_c_string(database_name) };

	match kind {
		StatementAuthorizerActionKind::Transaction
		| StatementAuthorizerActionKind::Savepoint => summary.transaction_control = true,
		StatementAuthorizerActionKind::Attach => summary.attach = true,
		StatementAuthorizerActionKind::Detach => summary.detach = true,
		StatementAuthorizerActionKind::Pragma => summary.pragma_usage = true,
		StatementAuthorizerActionKind::Function => summary.function_calls = true,
		_ => {}
	}

	if kind.is_schema_write() {
		summary.schema_writes = true;
	}
	if kind.is_temp_schema_write()
		|| (kind.is_data_write() && database_name.as_deref() == Some("temp"))
	{
		summary.temp_writes = true;
	}
	if kind.is_data_write() || kind.is_schema_write() || kind.is_temp_schema_write() {
		summary.write_operations = true;
	}

	summary.actions.push(StatementAuthorizerAction {
		kind,
		first_arg: unsafe { optional_c_string(first_arg) },
		second_arg: unsafe { optional_c_string(second_arg) },
		database_name,
		trigger_or_view_name: unsafe { optional_c_string(trigger_or_view_name) },
	});

	SQLITE_OK
}

unsafe extern "C" fn reader_authorizer_action(
	_user_data: *mut c_void,
	action_code: c_int,
	first_arg: *const c_char,
	second_arg: *const c_char,
	database_name: *const c_char,
	_trigger_or_view_name: *const c_char,
) -> c_int {
	let kind = StatementAuthorizerActionKind::from_code(action_code);
	let database_name = unsafe { optional_c_string(database_name) };
	let first_arg = unsafe { optional_c_string(first_arg) };
	let second_arg = unsafe { optional_c_string(second_arg) };

	if kind.is_data_write()
		|| kind.is_schema_write()
		|| kind.is_temp_schema_write()
		|| (kind.is_data_write() && database_name.as_deref() == Some("temp"))
	{
		return SQLITE_DENY;
	}

	match kind {
		StatementAuthorizerActionKind::Transaction
		| StatementAuthorizerActionKind::Savepoint
		| StatementAuthorizerActionKind::Attach
		| StatementAuthorizerActionKind::Detach => SQLITE_DENY,
		StatementAuthorizerActionKind::Pragma => {
			if reader_pragma_allowed(first_arg.as_deref(), second_arg.as_deref()) {
				SQLITE_OK
			} else {
				SQLITE_DENY
			}
		}
		StatementAuthorizerActionKind::Function => {
			if reader_function_allowed(first_arg.as_deref(), second_arg.as_deref()) {
				SQLITE_OK
			} else {
				SQLITE_DENY
			}
		}
		StatementAuthorizerActionKind::Read
		| StatementAuthorizerActionKind::Select
		| StatementAuthorizerActionKind::Other(_) => SQLITE_OK,
		StatementAuthorizerActionKind::Insert
		| StatementAuthorizerActionKind::Update
		| StatementAuthorizerActionKind::Delete
		| StatementAuthorizerActionKind::CreateIndex
		| StatementAuthorizerActionKind::CreateTable
		| StatementAuthorizerActionKind::CreateTrigger
		| StatementAuthorizerActionKind::CreateView
		| StatementAuthorizerActionKind::CreateVirtualTable
		| StatementAuthorizerActionKind::CreateTempIndex
		| StatementAuthorizerActionKind::CreateTempTable
		| StatementAuthorizerActionKind::CreateTempTrigger
		| StatementAuthorizerActionKind::CreateTempView
		| StatementAuthorizerActionKind::DropIndex
		| StatementAuthorizerActionKind::DropTable
		| StatementAuthorizerActionKind::DropTrigger
		| StatementAuthorizerActionKind::DropView
		| StatementAuthorizerActionKind::DropVirtualTable
		| StatementAuthorizerActionKind::DropTempIndex
		| StatementAuthorizerActionKind::DropTempTable
		| StatementAuthorizerActionKind::DropTempTrigger
		| StatementAuthorizerActionKind::DropTempView
		| StatementAuthorizerActionKind::AlterTable
		| StatementAuthorizerActionKind::Reindex
		| StatementAuthorizerActionKind::Analyze => SQLITE_DENY,
	}
}

fn reader_pragma_allowed(first_arg: Option<&str>, second_arg: Option<&str>) -> bool {
	let Some(name) = first_arg else {
		return false;
	};
	if second_arg.is_some() {
		return false;
	}

	matches!(
		name.to_ascii_lowercase().as_str(),
		"application_id"
			| "busy_timeout"
			| "cache_size"
			| "collation_list"
			| "compile_options"
			| "database_list"
			| "encoding"
			| "foreign_key_check"
			| "foreign_key_list"
			| "freelist_count"
			| "function_list"
			| "index_info"
			| "index_list"
			| "index_xinfo"
			| "integrity_check"
			| "journal_mode"
			| "module_list"
			| "page_count"
			| "page_size"
			| "pragma_list"
			| "quick_check"
			| "schema_version"
			| "table_info"
			| "table_list"
			| "table_xinfo"
			| "user_version"
	)
}

fn reader_function_allowed(first_arg: Option<&str>, second_arg: Option<&str>) -> bool {
	let name = second_arg.or(first_arg);
	!matches!(
		name.map(str::to_ascii_lowercase).as_deref(),
		Some("load_extension") | Some("writefile")
	)
}

unsafe fn optional_c_string(value: *const c_char) -> Option<String> {
	if value.is_null() {
		None
	} else {
		Some(
			unsafe { CStr::from_ptr(value) }
				.to_string_lossy()
				.into_owned(),
		)
	}
}

fn has_non_whitespace_tail(tail: *const c_char) -> bool {
	if tail.is_null() {
		return false;
	}

	let bytes = unsafe { CStr::from_ptr(tail).to_bytes() };
	bytes.iter().any(|byte| !byte.is_ascii_whitespace())
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
