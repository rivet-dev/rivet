use clickhouse_user_query::*;

#[test]
fn test_query_expr_serde() {
	let query = QueryExpr::StringEqual {
		property: "user_id".to_string(),
		subproperty: None,
		value: "12345".to_string(),
	};

	// Test serialization
	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""string_equal""#));
	assert!(json.contains(r#""property": "user_id""#));
	assert!(json.contains(r#""value": "12345""#));

	// Test deserialization
	let deserialized: QueryExpr = serde_json::from_str(&json).unwrap();
	match deserialized {
		QueryExpr::StringEqual {
			property, value, ..
		} => {
			assert_eq!(property, "user_id");
			assert_eq!(value, "12345");
		}
		_ => panic!("Expected StringEqual"),
	}
}

#[test]
fn test_complex_query_serde() {
	let query = QueryExpr::And {
		exprs: vec![
			QueryExpr::BoolEqual {
				property: "active".to_string(),
				subproperty: None,
				value: true,
			},
			QueryExpr::ArrayContains {
				property: "tags".to_string(),
				subproperty: Some("category".to_string()),
				values: vec!["premium".to_string(), "verified".to_string()],
			},
		],
	};

	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""and""#));
	assert!(json.contains(r#""bool_equal""#));
	assert!(json.contains(r#""array_contains""#));

	let deserialized: QueryExpr = serde_json::from_str(&json).unwrap();
	match deserialized {
		QueryExpr::And { exprs } => {
			assert_eq!(exprs.len(), 2);
		}
		_ => panic!("Expected And expression"),
	}
}

#[test]
fn test_query_expr_creation() {
	let query = QueryExpr::And {
		exprs: vec![
			QueryExpr::StringEqual {
				property: "user_id".to_string(),
				subproperty: None,
				value: "12345".to_string(),
			},
			QueryExpr::BoolEqual {
				property: "active".to_string(),
				subproperty: None,
				value: true,
			},
		],
	};

	match query {
		QueryExpr::And { exprs } => {
			assert_eq!(exprs.len(), 2);
		}
		_ => panic!("Expected And expression"),
	}
}

#[test]
fn test_subproperty_query() {
	let query = QueryExpr::StringEqual {
		property: "metadata".to_string(),
		subproperty: Some("key".to_string()),
		value: "value".to_string(),
	};

	match query {
		QueryExpr::StringEqual {
			property,
			subproperty,
			value,
		} => {
			assert_eq!(property, "metadata");
			assert_eq!(subproperty, Some("key".to_string()));
			assert_eq!(value, "value");
		}
		_ => panic!("Expected StringEqual expression"),
	}
}

#[test]
fn test_array_query() {
	let query = QueryExpr::ArrayContains {
		property: "tags".to_string(),
		subproperty: None,
		values: vec!["premium".to_string(), "verified".to_string()],
	};

	match query {
		QueryExpr::ArrayContains {
			property, values, ..
		} => {
			assert_eq!(property, "tags");
			assert_eq!(values.len(), 2);
			assert!(values.contains(&"premium".to_string()));
		}
		_ => panic!("Expected ArrayContains expression"),
	}
}

#[test]
fn test_numeric_query() {
	let query = QueryExpr::NumberGreater {
		property: "score".to_string(),
		subproperty: None,
		value: 85.5,
	};

	match query {
		QueryExpr::NumberGreater {
			property, value, ..
		} => {
			assert_eq!(property, "score");
			assert_eq!(value, 85.5);
		}
		_ => panic!("Expected NumberGreater expression"),
	}
}

#[test]
fn test_numeric_query_serde() {
	let query = QueryExpr::NumberLessOrEqual {
		property: "metrics".to_string(),
		subproperty: Some("latency".to_string()),
		value: 100.0,
	};

	// Test serialization
	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""number_less_or_equal""#));
	assert!(json.contains(r#""property": "metrics""#));
	assert!(json.contains(r#""subproperty": "latency""#));
	assert!(json.contains(r#""value": 100.0"#));

	// Test deserialization
	let deserialized: QueryExpr = serde_json::from_str(&json).unwrap();
	match deserialized {
		QueryExpr::NumberLessOrEqual {
			property,
			subproperty,
			value,
		} => {
			assert_eq!(property, "metrics");
			assert_eq!(subproperty, Some("latency".to_string()));
			assert_eq!(value, 100.0);
		}
		_ => panic!("Expected NumberLessOrEqual"),
	}
}
