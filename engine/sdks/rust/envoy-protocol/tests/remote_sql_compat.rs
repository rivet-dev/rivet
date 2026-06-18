use anyhow::Result;
use rivet_envoy_protocol::{
	generated::{v4, v6},
	versioned::{
		ProtocolCompatibilityDirection, ProtocolCompatibilityError, ProtocolCompatibilityFeature,
		ToEnvoy, ToRivet,
	},
};
use vbare::OwnedVersionedData;

fn remote_sql_request_exec() -> v6::ToRivet {
	v6::ToRivet::ToRivetSqliteExecRequest(v6::ToRivetSqliteExecRequest {
		request_id: 1,
		data: v6::SqliteExecRequest {
			namespace_id: "namespace".into(),
			actor_id: "actor".into(),
			generation: 7,
			sql: "select 1".into(),
		},
	})
}

fn remote_sql_request_execute() -> v6::ToRivet {
	v6::ToRivet::ToRivetSqliteExecuteRequest(v6::ToRivetSqliteExecuteRequest {
		request_id: 2,
		data: v6::SqliteExecuteRequest {
			namespace_id: "namespace".into(),
			actor_id: "actor".into(),
			generation: 7,
			sql: "select ?".into(),
			params: Some(vec![v6::SqliteBindParam::SqliteValueInteger(
				v6::SqliteValueInteger { value: 1 },
			)]),
		},
	})
}

fn remote_sql_response_exec() -> v6::ToEnvoy {
	v6::ToEnvoy::ToEnvoySqliteExecResponse(v6::ToEnvoySqliteExecResponse {
		request_id: 1,
		data: v6::SqliteExecResponse::SqliteErrorResponse(v6::SqliteErrorResponse {
			group: "sqlite".into(),
			code: "remote_unavailable".into(),
			message: "remote sql execution is unavailable".into(),
			metadata: None,
		}),
	})
}

fn remote_sql_response_execute() -> v6::ToEnvoy {
	v6::ToEnvoy::ToEnvoySqliteExecuteResponse(v6::ToEnvoySqliteExecuteResponse {
		request_id: 2,
		data: v6::SqliteExecuteResponse::SqliteErrorResponse(v6::SqliteErrorResponse {
			group: "sqlite".into(),
			code: "remote_unavailable".into(),
			message: "remote sql execution is unavailable".into(),
			metadata: None,
		}),
	})
}

fn assert_compatibility_error(
	err: anyhow::Error,
	direction: ProtocolCompatibilityDirection,
	target_version: u16,
) {
	let err = err
		.downcast_ref::<ProtocolCompatibilityError>()
		.expect("expected structured protocol compatibility error");

	assert_eq!(
		err.feature,
		ProtocolCompatibilityFeature::RemoteSqliteExecution
	);
	assert_eq!(err.direction, direction);
	assert_eq!(err.required_version, 4);
	assert_eq!(err.target_version, target_version);
}

fn staged_commit_request_begin() -> v6::ToRivet {
	v6::ToRivet::ToRivetSqliteCommitStageBeginRequest(
		v6::ToRivetSqliteCommitStageBeginRequest {
			request_id: 3,
			data: v6::SqliteCommitStageBeginRequest {
				actor_id: "actor".into(),
				db_size_pages: 64,
				now_ms: 1_000,
				expected_generation: Some(7),
				expected_head_txid: Some(1),
				dirty_pgnos: vec![1, 2, 3],
			},
		},
	)
}

fn staged_commit_response_finalize() -> v6::ToEnvoy {
	v6::ToEnvoy::ToEnvoySqliteCommitStageFinalizeResponse(
		v6::ToEnvoySqliteCommitStageFinalizeResponse {
			request_id: 3,
			data: v6::SqliteCommitResponse::SqliteErrorResponse(v6::SqliteErrorResponse {
				group: "depot".into(),
				code: "sqlite_commit_page_limit_exceeded".into(),
				message: "SQLite transaction touched too many pages.".into(),
				metadata: Some(r#"{"max_dirty_pages":8192}"#.into()),
			}),
		},
	)
}

fn assert_stage_compatibility_error(
	err: anyhow::Error,
	direction: ProtocolCompatibilityDirection,
	target_version: u16,
) {
	let err = err
		.downcast_ref::<ProtocolCompatibilityError>()
		.expect("expected structured protocol compatibility error");

	assert_eq!(
		err.feature,
		ProtocolCompatibilityFeature::SqliteCommitStaging
	);
	assert_eq!(err.direction, direction);
	assert_eq!(err.required_version, 6);
	assert_eq!(err.target_version, target_version);
}

#[test]
fn old_core_new_pegboard_envoy_rejects_remote_sql_request() {
	let err = ToRivet::wrap_latest(remote_sql_request_exec())
		.serialize(3)
		.expect_err("remote SQL requests must not serialize below v4");

	assert_compatibility_error(err, ProtocolCompatibilityDirection::ToRivet, 3);
}

#[test]
fn staged_commit_messages_require_v6() {
	let request_err = ToRivet::wrap_latest(staged_commit_request_begin())
		.serialize(5)
		.expect_err("staged commit requests must not serialize below v6");
	let response_err = ToEnvoy::wrap_latest(staged_commit_response_finalize())
		.serialize(5)
		.expect_err("staged commit responses must not serialize below v6");

	assert_stage_compatibility_error(request_err, ProtocolCompatibilityDirection::ToRivet, 5);
	assert_stage_compatibility_error(response_err, ProtocolCompatibilityDirection::ToEnvoy, 5);
}

#[test]
fn staged_commit_error_metadata_roundtrips_in_v6() -> Result<()> {
	let response = ToEnvoy::wrap_latest(staged_commit_response_finalize()).serialize(6)?;
	let v6::ToEnvoy::ToEnvoySqliteCommitStageFinalizeResponse(decoded) =
		ToEnvoy::deserialize(&response, 6)?
	else {
		panic!("expected staged finalize response");
	};
	let v6::SqliteCommitResponse::SqliteErrorResponse(error) = decoded.data else {
		panic!("expected staged finalize error response");
	};
	assert_eq!(error.metadata, Some(r#"{"max_dirty_pages":8192}"#.into()));

	Ok(())
}

#[test]
fn new_core_old_pegboard_envoy_rejects_remote_sql_response() {
	let err = ToEnvoy::wrap_latest(remote_sql_response_exec())
		.serialize(3)
		.expect_err("remote SQL responses must not serialize below v4");

	assert_compatibility_error(err, ProtocolCompatibilityDirection::ToEnvoy, 3);
}

#[test]
fn old_core_old_pegboard_envoy_rejects_remote_sql_both_directions() {
	let request_err = ToRivet::wrap_latest(remote_sql_request_exec())
		.serialize(3)
		.expect_err("remote SQL requests must not serialize below v4");
	let response_err = ToEnvoy::wrap_latest(remote_sql_response_exec())
		.serialize(3)
		.expect_err("remote SQL responses must not serialize below v4");

	assert_compatibility_error(request_err, ProtocolCompatibilityDirection::ToRivet, 3);
	assert_compatibility_error(response_err, ProtocolCompatibilityDirection::ToEnvoy, 3);
}

#[test]
fn new_core_new_pegboard_envoy_allows_remote_sql_both_directions() -> Result<()> {
	let request = ToRivet::wrap_latest(remote_sql_request_exec()).serialize(4)?;
	let response = ToEnvoy::wrap_latest(remote_sql_response_exec()).serialize(4)?;

	assert!(matches!(
		ToRivet::deserialize(&request, 4)?,
		v6::ToRivet::ToRivetSqliteExecRequest(_)
	));
	assert!(matches!(
		ToEnvoy::deserialize(&response, 4)?,
		v6::ToEnvoy::ToEnvoySqliteExecResponse(_)
	));

	Ok(())
}

#[test]
fn v4_remote_sql_payloads_do_not_decode_as_v3() -> Result<()> {
	let request = serde_bare::to_vec(&v4::ToRivet::ToRivetSqliteExecRequest(
		v4::ToRivetSqliteExecRequest {
			request_id: 1,
			data: v4::SqliteExecRequest {
				namespace_id: "namespace".into(),
				actor_id: "actor".into(),
				generation: 7,
				sql: "select 1".into(),
			},
		},
	))?;
	let response = serde_bare::to_vec(&v4::ToEnvoy::ToEnvoySqliteExecResponse(
		v4::ToEnvoySqliteExecResponse {
			request_id: 1,
			data: v4::SqliteExecResponse::SqliteErrorResponse(v4::SqliteErrorResponse {
				message: "remote sql execution is unavailable".into(),
			}),
		},
	))?;

	assert!(ToRivet::deserialize(&request, 3).is_err());
	assert!(ToEnvoy::deserialize(&response, 3).is_err());

	Ok(())
}

#[test]
fn all_remote_sql_request_variants_require_v4() {
	for version in 1..4 {
		for request in [remote_sql_request_exec(), remote_sql_request_execute()] {
			let err = ToRivet::wrap_latest(request)
				.serialize(version)
				.expect_err("remote SQL request variant must not serialize below v4");

			assert_compatibility_error(err, ProtocolCompatibilityDirection::ToRivet, version);
		}
	}
}

#[test]
fn all_remote_sql_response_variants_require_v4() {
	for version in 1..4 {
		for response in [remote_sql_response_exec(), remote_sql_response_execute()] {
			let err = ToEnvoy::wrap_latest(response)
				.serialize(version)
				.expect_err("remote SQL response variant must not serialize below v4");

			assert_compatibility_error(err, ProtocolCompatibilityDirection::ToEnvoy, version);
		}
	}
}
