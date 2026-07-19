use super::*;

#[test]
fn schema_version_is_little_endian_i64() {
	let encoded = encode_schema_version(INTERNAL_SCHEMA_VERSION);
	assert_eq!(
		decode_schema_version(&encoded).unwrap(),
		INTERNAL_SCHEMA_VERSION
	);
}

#[test]
fn ladder_version_matches_migration_count() {
	assert_eq!(MIGRATIONS.len() as i64, INTERNAL_SCHEMA_VERSION);
}

#[test]
fn schema_sql_does_not_embed_workload_annotations() {
	for sql in MIGRATIONS
		.iter()
		.flat_map(|migration| migration.iter().copied())
		.chain([CREATE_META_TABLE])
	{
		assert!(
			!sql.contains("-- W["),
			"workload annotation leaked into SQL: {sql}"
		);
	}
}

#[test]
fn unpublished_schema_has_explicit_values_and_minimal_constraints() {
	let sql = MIGRATIONS
		.iter()
		.flat_map(|migration| migration.iter().copied())
		.collect::<Vec<_>>()
		.join("\n")
		.to_ascii_lowercase();
	assert!(!sql.contains(" default "), "internal columns must not use defaults");
	assert!(
		!sql.replace("check (id = 1)", "").contains("check"),
		"only the singleton id constraint is allowed"
	);
	assert!(sql.contains("kind             integer not null"));
	assert!(sql.contains("result         integer not null"));

	for statement in MIGRATIONS
		.iter()
		.flat_map(|migration| migration.iter().copied())
		.filter(|statement| statement.trim_start().starts_with("CREATE TABLE"))
	{
		assert!(statement.contains("STRICT"), "table is not STRICT: {statement}");
	}
}
