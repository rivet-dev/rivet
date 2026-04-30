use sqlite_storage::error::SqliteStorageError;

#[test]
fn pitr_errors_are_typed_and_downcastable() {
	let err: anyhow::Error = SqliteStorageError::ForkOutOfRetention.into();
	let storage_err = err
		.downcast_ref::<SqliteStorageError>()
		.expect("sqlite storage errors should remain typed");

	assert_eq!(storage_err, &SqliteStorageError::ForkOutOfRetention);
}

#[test]
fn pitr_errors_have_rivet_error_codes() {
	let cases = [
		(
			SqliteStorageError::ForkChainTooDeep,
			"fork_chain_too_deep",
		),
		(
			SqliteStorageError::NamespaceForkChainTooDeep,
			"namespace_fork_chain_too_deep",
		),
		(
			SqliteStorageError::ForkOutOfRetention,
			"fork_out_of_retention",
		),
		(SqliteStorageError::BookmarkExpired, "bookmark_expired"),
		(
			SqliteStorageError::BranchNotReachable,
			"branch_not_reachable",
		),
		(
			SqliteStorageError::ShardVersionCapExhausted,
			"shard_version_cap_exhausted",
		),
		(SqliteStorageError::TooManyPins, "too_many_pins"),
		(SqliteStorageError::DatabaseNotFound, "database_not_found"),
	];

	for (err, code) in cases {
		let rivet_err = rivet_error::RivetError::extract(&err.build());
		assert_eq!(rivet_err.group(), "sqlite_storage");
		assert_eq!(rivet_err.code(), code);
	}
}
