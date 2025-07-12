use clickhouse_user_query::*;

fn create_test_schema() -> Schema {
    Schema::new(vec![
        Property::new("prop_a".to_string(), false, PropertyType::String).unwrap(),
        Property::new("prop_b".to_string(), true, PropertyType::String).unwrap(),
        Property::new("bool_prop".to_string(), false, PropertyType::Bool).unwrap(),
        Property::new("number_prop".to_string(), false, PropertyType::Number).unwrap(),
        Property::new("array_prop".to_string(), false, PropertyType::ArrayString).unwrap(),
    ]).unwrap()
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
    assert!(matches!(result, Err(UserQueryError::PropertyTypeMismatch(_, _, _))));
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
    assert!(matches!(result, Err(UserQueryError::SubpropertiesNotSupported(_))));
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
    assert!(matches!(builder_result, Err(UserQueryError::PropertyNotFound(_))));
}

#[test]
fn test_subproperty_with_special_chars() {
    let schema = create_test_schema();
    let query = QueryExpr::StringEqual {
        property: "prop_b".to_string(), // This supports subproperties
        subproperty: Some("sub-with-dashes".to_string()),
        value: "foo".to_string(),
    };
    
    // Subproperties with special characters should work fine with Identifier escaping
    let builder_result = UserDefinedQueryBuilder::new(&schema, &query);
    assert!(builder_result.is_ok());
    
    let builder = builder_result.unwrap();
    assert_eq!(builder.where_expr(), "prop_b['sub-with-dashes'] = ?");
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
    let schema = Schema::new(vec![
        Property::new("metrics".to_string(), true, PropertyType::Number).unwrap(),
    ]).unwrap();
    
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
    assert!(matches!(result, Err(UserQueryError::PropertyTypeMismatch(_, _, _))));
}