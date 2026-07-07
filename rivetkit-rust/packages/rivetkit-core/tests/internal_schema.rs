use super::*;

#[test]
fn schema_version_is_little_endian_i64() {
	let encoded = encode_schema_version(INTERNAL_SCHEMA_VERSION);
	assert_eq!(decode_schema_version(&encoded).unwrap(), INTERNAL_SCHEMA_VERSION);
}

#[test]
fn ladder_version_matches_migration_count() {
	assert_eq!(MIGRATIONS.len() as i64, INTERNAL_SCHEMA_VERSION);
}

#[test]
fn every_create_table_has_write_pattern_comment() {
	for sql in MIGRATIONS
		.iter()
		.flat_map(|migration| migration.iter().copied())
		.chain([CREATE_META_TABLE])
	{
		if sql.contains("CREATE TABLE") {
			assert!(
				sql.trim_start().starts_with("-- W["),
				"missing write-pattern comment: {sql}",
			);
		}
	}
}
