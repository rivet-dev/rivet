use depot::error::SqliteStorageError;

#[test]
fn pitr_errors_are_typed_and_downcastable() {
	let err: anyhow::Error = SqliteStorageError::ForkOutOfRetention.into();
	let storage_err = err
		.downcast_ref::<SqliteStorageError>()
		.expect("depot errors should remain typed");

	assert_eq!(storage_err, &SqliteStorageError::ForkOutOfRetention);
}

#[test]
fn pitr_errors_have_rivet_error_codes() {
	let cases = [
		(SqliteStorageError::ForkChainTooDeep, "fork_chain_too_deep"),
		(
			SqliteStorageError::BucketForkChainTooDeep,
			"bucket_fork_chain_too_deep",
		),
		(
			SqliteStorageError::ForkOutOfRetention,
			"fork_out_of_retention",
		),
		(
			SqliteStorageError::RestoreTargetExpired,
			"restore_point_expired",
		),
		(
			SqliteStorageError::BranchNotReachable,
			"branch_not_reachable",
		),
		(
			SqliteStorageError::ShardVersionCapExhausted,
			"shard_version_cap_exhausted",
		),
		(SqliteStorageError::TooManyPins, "too_many_pins"),
		(
			SqliteStorageError::TooManyRestorePoints,
			"too_many_restore_points",
		),
		(
			SqliteStorageError::InvalidPolicyValue {
				policy: "pitr",
				field: "interval_ms",
				value: 0,
			},
			"invalid_policy_value",
		),
		(SqliteStorageError::DatabaseNotFound, "database_not_found"),
	];

	for (err, code) in cases {
		let rivet_err = rivet_error::RivetError::extract(&err.build());
		assert_eq!(rivet_err.group(), "depot");
		assert_eq!(rivet_err.code(), code);
	}
}
