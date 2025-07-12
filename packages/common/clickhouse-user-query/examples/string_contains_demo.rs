use clickhouse_user_query::*;

fn main() {
	// Create a schema with string properties
	let schema = Schema::new(vec![
		Property::new("title".to_string(), false, PropertyType::String).unwrap(),
		Property::new("description".to_string(), false, PropertyType::String).unwrap(),
		Property::new("tags".to_string(), true, PropertyType::String).unwrap(),
	])
	.unwrap();

	println!("=== String Contains Demo ===\n");

	// Example 1: Case-sensitive contains
	println!("1. Case-sensitive contains:");
	let query1 = QueryExpr::StringContains {
		property: "title".to_string(),
		map_key: None,
		value: "Rust".to_string(),
		case_insensitive: false,
	};
	let builder1 = UserDefinedQueryBuilder::new(&schema, Some(&query1)).unwrap();
	println!("   Query: {}", builder1.where_expr());
	println!("   -> Will match: 'Rust Programming', 'Learning Rust'");
	println!("   -> Won't match: 'rust basics', 'RUST tutorial'\n");

	// Example 2: Case-insensitive contains
	println!("2. Case-insensitive contains:");
	let query2 = QueryExpr::StringContains {
		property: "title".to_string(),
		map_key: None,
		value: "rust".to_string(),
		case_insensitive: true,
	};
	let builder2 = UserDefinedQueryBuilder::new(&schema, Some(&query2)).unwrap();
	println!("   Query: {}", builder2.where_expr());
	println!("   -> Will match: 'Rust Programming', 'rust basics', 'RUST tutorial', 'RuSt'\n");

	// Example 3: Contains with special characters
	println!("3. Contains with special characters:");
	let query3 = QueryExpr::StringContains {
		property: "description".to_string(),
		map_key: None,
		value: "50% faster".to_string(),
		case_insensitive: false,
	};
	let builder3 = UserDefinedQueryBuilder::new(&schema, Some(&query3)).unwrap();
	println!("   Query: {}", builder3.where_expr());
	println!("   -> Special chars (%, _, \\) are properly escaped");
	println!("   -> Will match: 'This is 50% faster than before'\n");

	// Example 4: Contains with map key
	println!("4. Contains with map key:");
	let query4 = QueryExpr::StringContains {
		property: "tags".to_string(),
		map_key: Some("category".to_string()),
		value: "prog".to_string(),
		case_insensitive: true,
	};
	let builder4 = UserDefinedQueryBuilder::new(&schema, Some(&query4)).unwrap();
	println!("   Query: {}", builder4.where_expr());
	println!("   -> Will match tags['category'] containing 'prog', 'Prog', 'PROG'\n");

	// Example 5: Complex query with contains
	println!("5. Complex query combining contains with other operators:");
	let query5 = QueryExpr::And {
		exprs: vec![
			QueryExpr::StringContains {
				property: "title".to_string(),
				map_key: None,
				value: "guide".to_string(),
				case_insensitive: true,
			},
			QueryExpr::Or {
				exprs: vec![
					QueryExpr::StringContains {
						property: "description".to_string(),
						map_key: None,
						value: "beginner".to_string(),
						case_insensitive: true,
					},
					QueryExpr::StringContains {
						property: "description".to_string(),
						map_key: None,
						value: "tutorial".to_string(),
						case_insensitive: true,
					},
				],
			},
		],
	};
	let builder5 = UserDefinedQueryBuilder::new(&schema, Some(&query5)).unwrap();
	println!("   Query: {}", builder5.where_expr());
	println!("   -> Title must contain 'guide' (any case)");
	println!("   -> Description must contain either 'beginner' or 'tutorial' (any case)");
}
