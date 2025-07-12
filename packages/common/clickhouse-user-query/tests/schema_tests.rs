use clickhouse_user_query::*;

#[test]
fn test_schema_creation() {
	let schema = Schema::new(vec![
		Property::new("valid_name".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(false),
		Property::new("another_valid_123".to_string(), true, PropertyType::Bool)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	assert_eq!(schema.properties.len(), 2);
	assert!(schema.get_property("valid_name").is_some());
	assert!(schema.get_property("nonexistent").is_none());
}

#[test]
fn test_invalid_property_name() {
	let result = Property::new("invalid-name".to_string(), false, PropertyType::String);
	assert!(result.is_err());
}

#[test]
fn test_property_type_names() {
	assert_eq!(PropertyType::Bool.type_name(), "bool");
	assert_eq!(PropertyType::String.type_name(), "string");
	assert_eq!(PropertyType::Number.type_name(), "number");
}

#[test]
fn test_property_is_map() {
	let non_map_prop = Property::new("prop1".to_string(), false, PropertyType::String)
		.unwrap()
		.with_group_by(false);
	let map_prop = Property::new("prop2".to_string(), true, PropertyType::String)
		.unwrap()
		.with_group_by(false);

	assert!(!non_map_prop.is_map);
	assert!(map_prop.is_map);
}
