use clickhouse_user_query::*;

fn create_test_schema() -> Schema {
	Schema::new(vec![
		Property::new("prop_a".to_string(), false, PropertyType::String).unwrap(),
		Property::new("prop_b".to_string(), true, PropertyType::String).unwrap(),
		Property::new("bool_prop".to_string(), false, PropertyType::Bool).unwrap(),
		Property::new("number_prop".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("array_prop".to_string(), false, PropertyType::ArrayString).unwrap(),
	])
	.unwrap()
}

#[test]
fn test_simple_string_equal() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_a".to_string(),
		subproperty: None,
		value: "foo".to_string(),
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "prop_a = ?");
}

#[test]
fn test_subproperty_access() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_b".to_string(),
		subproperty: Some("sub".to_string()),
		value: "bar".to_string(),
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "prop_b['sub'] = ?");
}

#[test]
fn test_and_query() {
	let schema = create_test_schema();
	let query = QueryExpr::And {
		exprs: vec![
			QueryExpr::StringEqual {
				property: "prop_a".to_string(),
				subproperty: None,
				value: "foo".to_string(),
			},
			QueryExpr::BoolEqual {
				property: "bool_prop".to_string(),
				subproperty: None,
				value: true,
			},
		],
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "(prop_a = ? AND bool_prop = ?)");
}

#[test]
fn test_array_contains() {
	let schema = create_test_schema();
	let query = QueryExpr::ArrayContains {
		property: "array_prop".to_string(),
		subproperty: None,
		values: vec!["val1".to_string(), "val2".to_string()],
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "hasAny(array_prop, ?)");
}

#[test]
fn test_property_not_found() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "nonexistent".to_string(),
		subproperty: None,
		value: "foo".to_string(),
	};

	let result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(matches!(result, Err(UserQueryError::PropertyNotFound(_))));
}

#[test]
fn test_type_mismatch() {
	let schema = create_test_schema();
	let query = QueryExpr::BoolEqual {
		property: "prop_a".to_string(), // This is a string property
		subproperty: None,
		value: true,
	};

	let result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(matches!(
		result,
		Err(UserQueryError::PropertyTypeMismatch(_, _, _))
	));
}

#[test]
fn test_subproperties_not_supported() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_a".to_string(), // This doesn't support subproperties
		subproperty: Some("sub".to_string()),
		value: "foo".to_string(),
	};

	let result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(matches!(
		result,
		Err(UserQueryError::SubpropertiesNotSupported(_))
	));
}

#[test]
fn test_invalid_property_name() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop-with-dashes".to_string(),
		subproperty: None,
		value: "foo".to_string(),
	};

	// Invalid property names are now caught as "not found" since schema validation
	// happens at schema creation time, not query time
	let builder_result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(matches!(
		builder_result,
		Err(UserQueryError::PropertyNotFound(_))
	));
}

#[test]
fn test_subproperty_with_safe_chars() {
	let schema = create_test_schema();
	let query = QueryExpr::StringEqual {
		property: "prop_b".to_string(), // This supports subproperties
		subproperty: Some("sub_with_underscores123".to_string()),
		value: "foo".to_string(),
	};

	// Subproperties with safe characters (alphanumeric + underscore) should work
	let builder_result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(builder_result.is_ok());

	let builder = builder_result.unwrap();
	assert_eq!(
		builder.where_expr(),
		"prop_b['sub_with_underscores123'] = ?"
	);
}

#[test]
fn test_empty_array_values() {
	let schema = create_test_schema();
	let query = QueryExpr::ArrayContains {
		property: "array_prop".to_string(),
		subproperty: None,
		values: vec![],
	};

	let result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(matches!(result, Err(UserQueryError::EmptyArrayValues(_))));
}

#[test]
fn test_number_greater() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberGreater {
		property: "number_prop".to_string(),
		subproperty: None,
		value: 42.5,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "number_prop > ?");
}

#[test]
fn test_number_less_or_equal() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberLessOrEqual {
		property: "number_prop".to_string(),
		subproperty: None,
		value: 100.0,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "number_prop <= ?");
}

#[test]
fn test_number_with_subproperty() {
	let schema = Schema::new(vec![Property::new(
		"metrics".to_string(),
		true,
		PropertyType::Number,
	)
	.unwrap()])
	.unwrap();

	let query = QueryExpr::NumberEqual {
		property: "metrics".to_string(),
		subproperty: Some("score".to_string()),
		value: 95.5,
	};

	let builder = UserDefinedQueryBuilder::new(&schema, &query).unwrap();
	assert_eq!(builder.where_expr(), "metrics['score'] = ?");
}

#[test]
fn test_number_type_mismatch() {
	let schema = create_test_schema();
	let query = QueryExpr::NumberGreater {
		property: "prop_a".to_string(), // This is a String type, not Number
		subproperty: None,
		value: 42.0,
	};

	let result = UserDefinedQueryBuilder::new(&schema, &query);
	assert!(matches!(
		result,
		Err(UserQueryError::PropertyTypeMismatch(_, _, _))
	));
}

#[test]
fn test_subproperty_validation_valid_names() {
	let schema = create_test_schema();

	// Valid subproperty names
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
			subproperty: Some(name.to_string()),
			value: "test".to_string(),
		};

		let result = UserDefinedQueryBuilder::new(&schema, &query);
		assert!(
			result.is_ok(),
			"Valid subproperty name '{}' should be accepted",
			name
		);
	}
}

#[test]
fn test_subproperty_validation_invalid_names() {
	let schema = create_test_schema();

	// Invalid subproperty names
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
			subproperty: Some(name.to_string()),
			value: "test".to_string(),
		};

		let result = UserDefinedQueryBuilder::new(&schema, &query);
		assert!(
			result.is_err(),
			"Invalid subproperty name '{}' should be rejected",
			name
		);
		assert!(
			matches!(result, Err(UserQueryError::InvalidSubpropertyName(_))),
			"Invalid subproperty name '{}' should return InvalidSubpropertyName error",
			name
		);
	}
}

#[test]
fn test_subproperty_validation_sql_injection_attempts() {
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
			subproperty: Some(attempt.to_string()),
			value: "test".to_string(),
		};

		let result = UserDefinedQueryBuilder::new(&schema, &query);
		assert!(
			result.is_err(),
			"SQL injection attempt '{}' should be rejected",
			attempt
		);
		assert!(
			matches!(result, Err(UserQueryError::InvalidSubpropertyName(_))),
			"SQL injection attempt '{}' should return InvalidSubpropertyName error",
			attempt
		);
	}
}
