use super::*;

#[test]
fn remote_backend_requires_declared_database_and_capability() {
	assert_eq!(
		select_sqlite_backend(true, true),
		SqliteBackend::RemoteEnvoy
	);

	#[cfg(feature = "sqlite-local")]
	{
		assert_eq!(
			select_sqlite_backend(true, false),
			SqliteBackend::LocalNative
		);
		assert_eq!(
			select_sqlite_backend(false, true),
			SqliteBackend::LocalNative
		);
	}

	#[cfg(not(feature = "sqlite-local"))]
	{
		assert_eq!(
			select_sqlite_backend(true, false),
			SqliteBackend::Unavailable
		);
		assert_eq!(
			select_sqlite_backend(false, true),
			SqliteBackend::Unavailable
		);
	}
}

#[test]
fn protocol_conversion_preserves_bind_and_result_values() {
	let params = protocol_bind_params(vec![
		BindParam::Null,
		BindParam::Integer(7),
		BindParam::Float(1.5),
		BindParam::Text("hello".to_owned()),
		BindParam::Blob(vec![1, 2, 3]),
	]);

	assert!(matches!(
		params[0],
		protocol::SqliteBindParam::SqliteValueNull
	));
	assert!(matches!(
		params[1],
		protocol::SqliteBindParam::SqliteValueInteger(protocol::SqliteValueInteger { value: 7 })
	));
	assert!(matches!(
		params[2],
		protocol::SqliteBindParam::SqliteValueFloat(protocol::SqliteValueFloat { value })
			if f64::from_bits(u64::from_be_bytes(value)) == 1.5
	));
	assert!(matches!(
		&params[3],
		protocol::SqliteBindParam::SqliteValueText(protocol::SqliteValueText { value })
			if value == "hello"
	));
	assert!(matches!(
		&params[4],
		protocol::SqliteBindParam::SqliteValueBlob(protocol::SqliteValueBlob { value })
			if value == &vec![1, 2, 3]
	));

	let result = execute_result_from_protocol(protocol::SqliteExecuteResult {
		columns: vec!["id".to_owned(), "score".to_owned()],
		rows: vec![vec![
			protocol::SqliteColumnValue::SqliteValueInteger(protocol::SqliteValueInteger {
				value: 9,
			}),
			protocol::SqliteColumnValue::SqliteValueFloat(protocol::SqliteValueFloat {
				value: 2.25_f64.to_bits().to_be_bytes(),
			}),
		]],
		changes: 3,
		last_insert_row_id: Some(11),
	});

	assert_eq!(result.columns, vec!["id", "score"]);
	assert_eq!(
		result.rows,
		vec![vec![ColumnValue::Integer(9), ColumnValue::Float(2.25)]]
	);
	assert_eq!(result.changes, 3);
	assert_eq!(result.last_insert_row_id, Some(11));
}

#[test]
fn remote_protocol_compatibility_errors_become_remote_unavailable() {
	let err = anyhow::anyhow!(protocol::versioned::ProtocolCompatibilityError {
		feature: protocol::versioned::ProtocolCompatibilityFeature::RemoteSqliteExecution,
		direction: protocol::versioned::ProtocolCompatibilityDirection::ToRivet,
		required_version: 4,
		target_version: 3,
	});

	let mapped = remote_request_error(err);
	let structured = rivet_error::RivetError::extract(&mapped);
	assert_eq!(structured.group(), "sqlite");
	assert_eq!(structured.code(), "remote_unavailable");
}

#[test]
fn remote_lost_response_errors_become_indeterminate_result() {
	let err = anyhow::anyhow!(
		rivet_envoy_client::utils::RemoteSqliteIndeterminateResultError {
			operation: "execute",
		}
	);

	let mapped = remote_request_error(err);
	let structured = rivet_error::RivetError::extract(&mapped);
	assert_eq!(structured.group(), "sqlite");
	assert_eq!(structured.code(), "remote_indeterminate_result");
}
