use clickhouse_user_query::*;

#[test]
fn test_query_expr_serde() {
	let query = QueryExpr::StringEqual {
		property: "user_id".to_string(),
		map_key: None,
		value: "12345".to_string(),
		case_insensitive: false,
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
				map_key: None,
				value: true,
			},
			QueryExpr::StringIn {
				property: "status".to_string(),
				map_key: None,
				values: vec!["premium".to_string(), "verified".to_string()],
				case_insensitive: false,
			},
		],
	};

	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""and""#));
	assert!(json.contains(r#""bool_equal""#));
	assert!(json.contains(r#""string_in""#));

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
				map_key: None,
				value: "12345".to_string(),
				case_insensitive: false,
			},
			QueryExpr::BoolEqual {
				property: "active".to_string(),
				map_key: None,
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
fn test_map_key_query() {
	let query = QueryExpr::StringEqual {
		property: "metadata".to_string(),
		map_key: Some("key".to_string()),
		value: "value".to_string(),
		case_insensitive: false,
	};

	match query {
		QueryExpr::StringEqual {
			property,
			map_key,
			value,
			..
		} => {
			assert_eq!(property, "metadata");
			assert_eq!(map_key, Some("key".to_string()));
			assert_eq!(value, "value");
		}
		_ => panic!("Expected StringEqual expression"),
	}
}

#[test]
fn test_numeric_query() {
	let query = QueryExpr::NumberGreater {
		property: "score".to_string(),
		map_key: None,
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
		map_key: Some("latency".to_string()),
		value: 100.0,
	};

	// Test serialization
	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""number_less_or_equal""#));
	assert!(json.contains(r#""property": "metrics""#));
	assert!(json.contains(r#""map_key": "latency""#));
	assert!(json.contains(r#""value": 100.0"#));

	// Test deserialization
	let deserialized: QueryExpr = serde_json::from_str(&json).unwrap();
	match deserialized {
		QueryExpr::NumberLessOrEqual {
			property,
			map_key,
			value,
		} => {
			assert_eq!(property, "metrics");
			assert_eq!(map_key, Some("latency".to_string()));
			assert_eq!(value, 100.0);
		}
		_ => panic!("Expected NumberLessOrEqual"),
	}
}

#[test]
fn test_string_in_query() {
	let query = QueryExpr::StringIn {
		property: "status".to_string(),
		map_key: None,
		values: vec!["active".to_string(), "pending".to_string()],
		case_insensitive: false,
	};

	match query {
		QueryExpr::StringIn {
			property, values, ..
		} => {
			assert_eq!(property, "status");
			assert_eq!(values.len(), 2);
			assert!(values.contains(&"active".to_string()));
		}
		_ => panic!("Expected StringIn expression"),
	}
}

#[test]
fn test_string_not_in_query() {
	let query = QueryExpr::StringNotIn {
		property: "status".to_string(),
		map_key: None,
		values: vec!["disabled".to_string(), "archived".to_string()],
		case_insensitive: false,
	};

	// Test serialization
	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""string_not_in""#));
	assert!(json.contains(r#""property": "status""#));
	assert!(json.contains(r#""disabled""#));

	// Test deserialization
	let deserialized: QueryExpr = serde_json::from_str(&json).unwrap();
	match deserialized {
		QueryExpr::StringNotIn {
			property, values, ..
		} => {
			assert_eq!(property, "status");
			assert_eq!(values.len(), 2);
		}
		_ => panic!("Expected StringNotIn"),
	}
}

#[test]
fn test_number_in_query() {
	let query = QueryExpr::NumberIn {
		property: "score".to_string(),
		map_key: None,
		values: vec![85.0, 90.0, 95.0],
	};

	match query {
		QueryExpr::NumberIn {
			property, values, ..
		} => {
			assert_eq!(property, "score");
			assert_eq!(values.len(), 3);
			assert!(values.contains(&90.0));
		}
		_ => panic!("Expected NumberIn expression"),
	}
}

#[test]
fn test_number_not_in_query() {
	let query = QueryExpr::NumberNotIn {
		property: "score".to_string(),
		map_key: None,
		values: vec![0.0, 50.0],
	};

	// Test serialization
	let json = serde_json::to_string_pretty(&query).unwrap();

	assert!(json.contains(r#""number_not_in""#));
	assert!(json.contains(r#""property": "score""#));
	assert!(json.contains(r#"0.0"#));
	assert!(json.contains(r#"50.0"#));

	// Test deserialization
	let deserialized: QueryExpr = serde_json::from_str(&json).unwrap();
	match deserialized {
		QueryExpr::NumberNotIn {
			property, values, ..
		} => {
			assert_eq!(property, "score");
			assert_eq!(values.len(), 2);
		}
		_ => panic!("Expected NumberNotIn"),
	}
}
