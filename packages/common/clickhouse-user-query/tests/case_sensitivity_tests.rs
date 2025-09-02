use clickhouse_user_query::*;

fn create_test_schema() -> Schema {
	Schema::new(vec![
		Property::new("name".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(false),
		Property::new("tags".to_string(), true, PropertyType::String)
			.unwrap()
			.with_group_by(false),
		Property::new("description".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap()
}

#[test]
fn test_string_equal_case_sensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "name".to_string(),
		map_key: None,
		value: "Test".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "name = ?");
}

#[test]
fn test_string_equal_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "name".to_string(),
		map_key: None,
		value: "Test".to_string(),
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "LOWER(name) = LOWER(?)");
}

#[test]
fn test_string_not_equal_case_sensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringNotEqual {
		property: "name".to_string(),
		map_key: None,
		value: "Test".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "name != ?");
}

#[test]
fn test_string_not_equal_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringNotEqual {
		property: "name".to_string(),
		map_key: None,
		value: "Test".to_string(),
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "LOWER(name) != LOWER(?)");
}

#[test]
fn test_string_in_case_sensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringIn {
		property: "name".to_string(),
		map_key: None,
		values: vec!["Foo".to_string(), "Bar".to_string()],
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "name IN (?, ?)");
}

#[test]
fn test_string_in_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringIn {
		property: "name".to_string(),
		map_key: None,
		values: vec!["Foo".to_string(), "Bar".to_string()],
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "LOWER(name) IN (LOWER(?), LOWER(?))");
}

#[test]
fn test_string_not_in_case_sensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringNotIn {
		property: "name".to_string(),
		map_key: None,
		values: vec!["Foo".to_string(), "Bar".to_string()],
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "name NOT IN (?, ?)");
}

#[test]
fn test_string_not_in_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringNotIn {
		property: "name".to_string(),
		map_key: None,
		values: vec!["Foo".to_string(), "Bar".to_string()],
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(
		builder.where_expr(),
		"LOWER(name) NOT IN (LOWER(?), LOWER(?))"
	);
}

#[test]
fn test_string_match_regex_case_sensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringMatchRegex {
		property: "description".to_string(),
		map_key: None,
		pattern: "^Test.*end$".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "match(description, ?)");
}

#[test]
fn test_string_match_regex_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringMatchRegex {
		property: "description".to_string(),
		map_key: None,
		pattern: "^test.*end$".to_string(),
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	// The pattern should have (?i) prepended for case-insensitive matching
	assert_eq!(builder.where_expr(), "match(description, ?)");
	// Note: The actual pattern bound will be "(?i)^test.*end$"
}

#[test]
fn test_map_key_with_case_sensitivity() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "tags".to_string(),
		map_key: Some("category".to_string()),
		value: "Important".to_string(),
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "LOWER(tags['category']) = LOWER(?)");
}

#[test]
fn test_regex_with_map_key() {
	let schema = create_test_schema();
	let query = QueryExpr::StringMatchRegex {
		property: "tags".to_string(),
		map_key: Some("status".to_string()),
		pattern: "active|pending".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "match(tags['status'], ?)");
}

#[test]
fn test_complex_query_with_mixed_case_sensitivity() {
	let schema = create_test_schema();
	let query = QueryExpr::And {
		exprs: vec![
			QueryExpr::StringEqual {
				property: "name".to_string(),
				map_key: None,
				value: "TestUser".to_string(),
				case_insensitive: false, // exact match
			},
			QueryExpr::StringMatchRegex {
				property: "description".to_string(),
				map_key: None,
				pattern: "admin|manager".to_string(),
				case_insensitive: true, // case-insensitive regex
			},
		],
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "(name = ? AND match(description, ?))");
}

#[test]
fn test_string_in_with_empty_values_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringIn {
		property: "name".to_string(),
		map_key: None,
		values: vec![],
		case_insensitive: true,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(result, Err(UserQueryError::EmptyArrayValues(_))));
}

#[test]
fn test_regex_property_type_mismatch() {
	let schema = Schema::new(vec![
		Property::new("count".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	let query = QueryExpr::StringMatchRegex {
		property: "count".to_string(), // This is a number property
		map_key: None,
		pattern: "\\d+".to_string(),
		case_insensitive: false,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(
		result,
		Err(UserQueryError::PropertyTypeMismatch(_, _, _))
	));
}

#[test]
fn test_string_contains_case_sensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringContains {
		property: "name".to_string(),
		map_key: None,
		value: "Test".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "name LIKE ?");
}

#[test]
fn test_string_contains_case_insensitive() {
	let schema = create_test_schema();
	let query = QueryExpr::StringContains {
		property: "name".to_string(),
		map_key: None,
		value: "Test".to_string(),
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "name ILIKE ?");
}

#[test]
fn test_string_contains_with_special_characters() {
	let schema = create_test_schema();
	let query = QueryExpr::StringContains {
		property: "description".to_string(),
		map_key: None,
		value: "test%pattern_with\\backslash".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "description LIKE ?");
	// The actual bound value will be "%test\\%pattern\\_with\\\\backslash%"
}

#[test]
fn test_string_contains_with_map_key() {
	let schema = create_test_schema();
	let query = QueryExpr::StringContains {
		property: "tags".to_string(),
		map_key: Some("category".to_string()),
		value: "Important".to_string(),
		case_insensitive: true,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "tags['category'] ILIKE ?");
}
