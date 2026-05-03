//! Shared SQLite execution types for local and remote depot client backends.

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
pub struct ExecuteResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
	pub changes: i64,
	pub last_insert_row_id: Option<i64>,
}

impl ExecuteResult {
	pub fn into_query_result(self) -> QueryResult {
		QueryResult {
			columns: self.columns,
			rows: self.rows,
		}
	}

	pub fn into_exec_result(self) -> ExecResult {
		ExecResult {
			changes: self.changes,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColumnValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

#[cfg(test)]
mod tests {
	use super::{ColumnValue, ExecuteResult};

	#[test]
	fn execute_result_preserves_result_and_route_metadata() {
		let result = ExecuteResult {
			columns: vec!["id".to_owned(), "name".to_owned()],
			rows: vec![vec![
				ColumnValue::Integer(7),
				ColumnValue::Text("alpha".to_owned()),
			]],
			changes: 3,
			last_insert_row_id: Some(42),
		};

		assert_eq!(result.columns, vec!["id", "name"]);
		assert_eq!(
			result.rows,
			vec![vec![
				ColumnValue::Integer(7),
				ColumnValue::Text("alpha".to_owned())
			]]
		);
		assert_eq!(result.changes, 3);
		assert_eq!(result.last_insert_row_id, Some(42));
	}

	#[test]
	fn execute_result_projects_query_and_exec_results() {
		let result = ExecuteResult {
			columns: vec!["count".to_owned()],
			rows: vec![vec![ColumnValue::Integer(9)]],
			changes: 2,
			last_insert_row_id: Some(10),
		};

		let query_result = result.clone().into_query_result();
		assert_eq!(query_result.columns, vec!["count"]);
		assert_eq!(query_result.rows, vec![vec![ColumnValue::Integer(9)]]);

		let exec_result = result.into_exec_result();
		assert_eq!(exec_result.changes, 2);
	}
}
