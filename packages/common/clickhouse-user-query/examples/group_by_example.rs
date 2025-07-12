use clickhouse_user_query::*;

fn main() {
	// Define a schema with properties that can be grouped by
	let schema = Schema::new(vec![
		Property::new("datacenter_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("tags".to_string(), true, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("cpu_millicores".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(false),
	])
	.unwrap();

	// Example 1: Group by simple property
	let query_expr = QueryExpr::NumberGreater {
		property: "cpu_millicores".to_string(),
		map_key: None,
		value: 1000.0,
	};

	let key_path = KeyPath::new("datacenter_id".to_string());
	let builder =
		UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query_expr), Some(&key_path))
			.unwrap();

	println!("Simple property GROUP BY:");
	println!("WHERE clause: {}", builder.where_expr());
	println!("GROUP BY clause: {:?}", builder.group_by_expr());
	println!();

	// Example 2: Group by map property with key
	let key_path = KeyPath::with_map_key("tags".to_string(), "region".to_string());
	let builder =
		UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query_expr), Some(&key_path))
			.unwrap();

	println!("Map property with key GROUP BY:");
	println!("WHERE clause: {}", builder.where_expr());
	println!("GROUP BY clause: {:?}", builder.group_by_expr());
	println!();

	// Example 3: No group by
	let builder =
		UserDefinedQueryBuilder::new_with_group_by(&schema, Some(&query_expr), None).unwrap();

	println!("No GROUP BY:");
	println!("WHERE clause: {}", builder.where_expr());
	println!("GROUP BY clause: {:?}", builder.group_by_expr());
}
