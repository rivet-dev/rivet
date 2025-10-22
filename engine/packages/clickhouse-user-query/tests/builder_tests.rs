use clickhouse_user_query::*;

fn create_test_schema() -> Schema {
	Schema::new(vec![
		Property::new("prop_a".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(false),
		Property::new("prop_b".to_string(), true, PropertyType::String)
			.unwrap()
			.with_group_by(false),
		Property::new("bool_prop".to_string(), false, PropertyType::Bool)
			.unwrap()
			.with_group_by(false),
		Property::new("number_prop".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap()
}

#[test]
fn test_simple_string_equal() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_a".to_string(),
		map_key: None,
		value: "foo".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "prop_a = ?");
}

#[test]
fn test_map_key_access() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_b".to_string(),
		map_key: Some("sub".to_string()),
		value: "bar".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "prop_b['sub'] = ?");
}

#[test]
fn test_and_query() {
	let schema = create_test_schema();
	let query = QueryExpr::And {
		exprs: vec![
			QueryExpr::StringEqual {
				property: "prop_a".to_string(),
				map_key: None,
				value: "foo".to_string(),
				case_insensitive: false,
			},
			QueryExpr::BoolEqual {
				property: "bool_prop".to_string(),
				map_key: None,
				value: true,
			},
		],
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "(prop_a = ? AND bool_prop = ?)");
}

#[test]
fn test_property_not_found() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "nonexistent".to_string(),
		map_key: None,
		value: "foo".to_string(),
		case_insensitive: false,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(result, Err(UserQueryError::PropertyNotFound(_))));
}

#[test]
fn test_type_mismatch() {
	let schema = create_test_schema();
	let query = QueryExpr::BoolEqual {
		property: "prop_a".to_string(), // This is a string property
		map_key: None,
		value: true,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(
		result,
		Err(UserQueryError::PropertyTypeMismatch(_, _, _))
	));
}

#[test]
fn test_map_keys_not_supported() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_a".to_string(), // This doesn't support map keys
		map_key: Some("sub".to_string()),
		value: "foo".to_string(),
		case_insensitive: false,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(
		result,
		Err(UserQueryError::MapKeysNotSupported(_))
	));
}

#[test]
fn test_invalid_property_name() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop-with-dashes".to_string(),
		map_key: None,
		value: "foo".to_string(),
		case_insensitive: false,
	};

	// Invalid property names are now caught as "not found" since schema validation
	// happens at schema creation time, not query time
	let builder_result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(
		builder_result,
		Err(UserQueryError::PropertyNotFound(_))
	));
}

#[test]
fn test_map_key_with_safe_chars() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_b".to_string(), // This supports map keys
		map_key: Some("sub_with_underscores123".to_string()),
		value: "foo".to_string(),
		case_insensitive: false,
	};

	// Map keys with safe characters (alphanumeric + underscore) should work
	let builder_result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(builder_result.is_ok());

	let builder = builder_result.unwrap();
	assert_eq!(
		builder.where_expr(),
		"prop_b['sub_with_underscores123'] = ?"
	);
}

#[test]
fn test_number_greater() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberGreater {
		property: "number_prop".to_string(),
		map_key: None,
		value: 42.5,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "number_prop > ?");
}

#[test]
fn test_number_less_or_equal() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberLessOrEqual {
		property: "number_prop".to_string(),
		map_key: None,
		value: 100.0,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "number_prop <= ?");
}

#[test]
fn test_number_with_map_key() {
	let schema = Schema::new(vec![
		Property::new("metrics".to_string(), true, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	let query = QueryExpr::NumberEqual {
		property: "metrics".to_string(),
		map_key: Some("score".to_string()),
		value: 95.5,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "metrics['score'] = ?");
}

#[test]
fn test_number_type_mismatch() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberGreater {
		property: "prop_a".to_string(), // This is a String type, not Number
		map_key: None,
		value: 42.0,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(
		result,
		Err(UserQueryError::PropertyTypeMismatch(_, _, _))
	));
}

#[test]
fn test_map_key_validation_valid_names() {
	let schema = create_test_schema();

	// Valid map key names
	let valid_names = vec![
		"valid_name",
		"valid123",
		"name_with_underscore",
		"CamelCase",
		"a",
		"a1",
		"test_case_123",
	];

	for name in valid_names {
		let query = QueryExpr::StringEqual {
			property: "prop_b".to_string(),
			map_key: Some(name.to_string()),
			value: "test".to_string(),
			case_insensitive: false,
		};

		let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
		assert!(
			result.is_ok(),
			"Valid map key name '{}' should be accepted",
			name
		);
	}
}

#[test]
fn test_map_key_validation_invalid_names() {
	let schema = create_test_schema();

	// Invalid map key names
	let long_name = "a".repeat(65);
	let invalid_names = vec![
		"",              // Empty
		"123invalid",    // Starts with number
		"invalid-name",  // Contains dash
		"invalid.name",  // Contains dot
		"invalid name",  // Contains space
		"invalid@name",  // Contains special character
		"invalid'name",  // Contains quote
		"invalid\"name", // Contains double quote
		"invalid;name",  // Contains semicolon
		"invalid(name",  // Contains parenthesis
		"invalid)name",  // Contains parenthesis
		"invalid[name",  // Contains bracket
		"invalid]name",  // Contains bracket
		"invalid{name",  // Contains brace
		"invalid}name",  // Contains brace
		"invalid+name",  // Contains plus
		"invalid=name",  // Contains equals
		"invalid*name",  // Contains asterisk
		"invalid%name",  // Contains percent
		"invalid$name",  // Contains dollar
		"invalid#name",  // Contains hash
		"invalid!name",  // Contains exclamation
		"invalid|name",  // Contains pipe
		"invalid\\name", // Contains backslash
		"invalid/name",  // Contains forward slash
		"invalid<name",  // Contains less than
		"invalid>name",  // Contains greater than
		"invalid?name",  // Contains question mark
		"invalid~name",  // Contains tilde
		"invalid`name",  // Contains backtick
		&long_name,      // Too long (over 64 characters)
	];

	for name in invalid_names {
		let query = QueryExpr::StringEqual {
			property: "prop_b".to_string(),
			map_key: Some(name.to_string()),
			value: "test".to_string(),
			case_insensitive: false,
		};

		let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
		assert!(
			result.is_err(),
			"Invalid map key name '{}' should be rejected",
			name
		);
		assert!(
			matches!(result, Err(UserQueryError::InvalidMapKeyName(_))),
			"Invalid map key name '{}' should return InvalidMapKeyName error",
			name
		);
	}
}

#[test]
fn test_map_key_validation_sql_injection_attempts() {
	let schema = create_test_schema();

	// SQL injection attempts
	let injection_attempts = vec![
		"'; DROP TABLE users; --",
		"' OR '1'='1",
		"'; INSERT INTO users VALUES ('hacker'); --",
		"' UNION SELECT * FROM passwords --",
		"'; DELETE FROM users; --",
		"'/**/OR/**/1=1",
		"' OR 1=1 #",
		"admin'--",
		"' OR 'x'='x",
		"1' OR '1'='1",
		"'; EXEC xp_cmdshell('dir'); --",
		"'; SHUTDOWN; --",
	];

	for attempt in injection_attempts {
		let query = QueryExpr::StringEqual {
			property: "prop_b".to_string(),
			map_key: Some(attempt.to_string()),
			value: "test".to_string(),
			case_insensitive: false,
		};

		let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
		assert!(
			result.is_err(),
			"SQL injection attempt '{}' should be rejected",
			attempt
		);
		assert!(
			matches!(result, Err(UserQueryError::InvalidMapKeyName(_))),
			"SQL injection attempt '{}' should return InvalidMapKeyName error",
			attempt
		);
	}
}

#[test]
fn test_string_in() {
	let schema = create_test_schema();
	let query = QueryExpr::StringIn {
		property: "prop_a".to_string(),
		map_key: None,
		values: vec!["foo".to_string(), "bar".to_string()],
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "prop_a IN (?, ?)");
}

#[test]
fn test_string_not_in() {
	let schema = create_test_schema();
	let query = QueryExpr::StringNotIn {
		property: "prop_a".to_string(),
		map_key: None,
		values: vec!["foo".to_string(), "bar".to_string()],
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "prop_a NOT IN (?, ?)");
}

#[test]
fn test_number_in() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberIn {
		property: "number_prop".to_string(),
		map_key: None,
		values: vec![42.0, 84.0, 168.0],
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "number_prop IN (?, ?, ?)");
}

#[test]
fn test_number_not_in() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberNotIn {
		property: "number_prop".to_string(),
		map_key: None,
		values: vec![1.0, 2.0],
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "number_prop NOT IN (?, ?)");
}

#[test]
fn test_string_in_with_map_key() {
	let schema = Schema::new(vec![
		Property::new("metadata".to_string(), true, PropertyType::String)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	let query = QueryExpr::StringIn {
		property: "metadata".to_string(),
		map_key: Some("category".to_string()),
		values: vec!["premium".to_string(), "basic".to_string()],
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, Some(&query)).unwrap();
	assert_eq!(builder.where_expr(), "metadata['category'] IN (?, ?)");
}

#[test]
fn test_empty_in_values() {
	let schema = create_test_schema();
	let query = QueryExpr::StringIn {
		property: "prop_a".to_string(),
		map_key: None,
		values: vec![],
		case_insensitive: false,
	};

	let result = UserDefinedQueryBuilder::new(&schema, Some(&query));
	assert!(matches!(result, Err(UserQueryError::EmptyArrayValues(_))));
}

#[test]
fn test_group_by_simple() {
	let schema = Schema::new(vec![
		Property::new("user_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("status".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("score".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	let query = QueryExpr::StringEqual {
		property: "status".to_string(),
		map_key: None,
		value: "active".to_string(),
		case_insensitive: false,
	};

	let key_path = KeyPath::new("user_id".to_string());
	let builder =
		UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query), Some(&key_path)).unwrap();
	assert_eq!(builder.where_expr(), "status = ?");
	assert_eq!(builder.group_by_expr(), Some("user_id"));
}

#[test]
fn test_group_by_multiple_columns() {
	let schema = Schema::new(vec![
		Property::new("user_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("status".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("region".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
	])
	.unwrap();

	let query = QueryExpr::StringEqual {
		property: "status".to_string(),
		map_key: None,
		value: "active".to_string(),
		case_insensitive: false,
	};

	// Note: new_with_group_by only supports a single group by column
	let key_path = KeyPath::new("user_id".to_string());
	let builder =
		UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query), Some(&key_path)).unwrap();
	assert_eq!(builder.group_by_expr(), Some("user_id"));
}

#[test]
fn test_group_by_property_not_groupable() {
	let schema = Schema::new(vec![
		Property::new("user_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("score".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	let query = QueryExpr::NumberGreater {
		property: "score".to_string(),
		map_key: None,
		value: 50.0,
	};

	let key_path = KeyPath::new("score".to_string());
	let result = UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query), Some(&key_path));
	assert!(matches!(
		result,
		Err(UserQueryError::PropertyCannotBeGroupedBy(_))
	));
}

#[test]
fn test_group_by_map_property_rejected() {
	let schema = Schema::new(vec![
		Property::new("user_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("metadata".to_string(), true, PropertyType::String)
			.unwrap()
			.with_group_by(true),
	])
	.unwrap();

	let query = QueryExpr::StringEqual {
		property: "user_id".to_string(),
		map_key: None,
		value: "user123".to_string(),
		case_insensitive: false,
	};

	let key_path = KeyPath::new("metadata".to_string());
	let result = UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query), Some(&key_path));
	assert!(matches!(
		result,
		Err(UserQueryError::MapPropertyCannotBeGroupedBy(_))
	));
}

#[test]
fn test_group_by_property_not_found() {
	let schema = Schema::new(vec![
		Property::new("user_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
	])
	.unwrap();

	let query = QueryExpr::StringEqual {
		property: "user_id".to_string(),
		map_key: None,
		value: "user123".to_string(),
		case_insensitive: false,
	};

	let key_path = KeyPath::new("unknown_column".to_string());
	let result = UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query), Some(&key_path));
	assert!(matches!(result, Err(UserQueryError::PropertyNotFound(_))));
}

#[test]
fn test_group_by_empty_allowed() {
	let schema = Schema::new(vec![
		Property::new("user_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
	])
	.unwrap();

	let query = QueryExpr::StringEqual {
		property: "user_id".to_string(),
		map_key: None,
		value: "user123".to_string(),
		case_insensitive: false,
	};

	let builder = UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query), None).unwrap();
	assert_eq!(builder.group_by_expr(), None);
}

#[test]
fn test_none_expr_returns_true() {
	// Create schema with groupable property
	let schema = Schema::new(vec![
		Property::new("prop_a".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("prop_b".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	// Test with None expression
	let builder = UserDefinedQueryBuilder::new(&schema, None).unwrap();
	assert_eq!(builder.where_expr(), "true");

	// Test with None expression and group by
	let key_path = KeyPath::new("prop_a".to_string());
	let builder_with_group =
		UserDefinedQueryBuilder::new_with_group_by(&schema, None, Some(&key_path)).unwrap();
	assert_eq!(builder_with_group.where_expr(), "true");
	assert_eq!(builder_with_group.group_by_expr(), Some("prop_a"));
}
