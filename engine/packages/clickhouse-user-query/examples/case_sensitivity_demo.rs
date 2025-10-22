use clickhouse_user_query::*;

fn main() {
	// Create a schema with string properties
	let schema = Schema::new(vec![
		Property::new("username".to_string(), false, PropertyType::String).unwrap(),
		Property::new("email".to_string(), false, PropertyType::String).unwrap(),
		Property::new("tags".to_string(), true, PropertyType::String).unwrap(),
	])
	.unwrap();

	println!("=== Case Sensitivity Demo ===\n");

	// Example 1: Case-sensitive string equality
	println!("1. Case-sensitive equality:");
	let query1 = QueryExpr::StringEqual {
		property: "username".to_string(),
		map_key: None,
		value: "JohnDoe".to_string(),
		case_insensitive: false,
	};
	let builder1 = UserDefinedQueryBuilder::new(&schema, Some(&query1)).unwrap();
	println!("   Query: {}", builder1.where_expr());
	println!("   -> Will match: 'JohnDoe'");
	println!("   -> Won't match: 'johndoe', 'JOHNDOE'\n");

	// Example 2: Case-insensitive string equality
	println!("2. Case-insensitive equality:");
	let query2 = QueryExpr::StringEqual {
		property: "username".to_string(),
		map_key: None,
		value: "JohnDoe".to_string(),
		case_insensitive: true,
	};
	let builder2 = UserDefinedQueryBuilder::new(&schema, Some(&query2)).unwrap();
	println!("   Query: {}", builder2.where_expr());
	println!("   -> Will match: 'JohnDoe', 'johndoe', 'JOHNDOE', 'jOhNdOe'\n");

	// Example 3: Case-sensitive regex matching
	println!("3. Case-sensitive regex:");
	let query3 = QueryExpr::StringMatchRegex {
		property: "email".to_string(),
		map_key: None,
		pattern: "^[A-Z].*@example\\.com$".to_string(),
		case_insensitive: false,
	};
	let builder3 = UserDefinedQueryBuilder::new(&schema, Some(&query3)).unwrap();
	println!("   Query: {}", builder3.where_expr());
	println!("   Pattern: ^[A-Z].*@example\\.com$");
	println!("   -> Will match: 'Admin@example.com'");
	println!("   -> Won't match: 'admin@example.com'\n");

	// Example 4: Case-insensitive regex matching
	println!("4. Case-insensitive regex:");
	let query4 = QueryExpr::StringMatchRegex {
		property: "email".to_string(),
		map_key: None,
		pattern: "admin|support".to_string(),
		case_insensitive: true,
	};
	let builder4 = UserDefinedQueryBuilder::new(&schema, Some(&query4)).unwrap();
	println!("   Query: {}", builder4.where_expr());
	println!("   Pattern: admin|support (with (?i) prefix)");
	println!("   -> Will match: 'admin@test.com', 'ADMIN@test.com', 'Support@test.com'\n");

	// Example 5: Case-insensitive IN clause
	println!("5. Case-insensitive IN clause:");
	let query5 = QueryExpr::StringIn {
		property: "username".to_string(),
		map_key: None,
		values: vec!["Admin".to_string(), "Support".to_string()],
		case_insensitive: true,
	};
	let builder5 = UserDefinedQueryBuilder::new(&schema, Some(&query5)).unwrap();
	println!("   Query: {}", builder5.where_expr());
	println!("   -> Will match: 'admin', 'ADMIN', 'support', 'SUPPORT'\n");

	// Example 6: Complex query with mixed case sensitivity
	println!("6. Complex query with mixed sensitivity:");
	let query6 = QueryExpr::And {
		exprs: vec![
			QueryExpr::StringEqual {
				property: "username".to_string(),
				map_key: None,
				value: "Admin".to_string(),
				case_insensitive: true, // Case-insensitive username
			},
			QueryExpr::StringMatchRegex {
				property: "tags".to_string(),
				map_key: Some("role".to_string()),
				pattern: "^(Admin|Manager)$".to_string(),
				case_insensitive: false, // Case-sensitive role
			},
		],
	};
	let builder6 = UserDefinedQueryBuilder::new(&schema, Some(&query6)).unwrap();
	println!("   Query: {}", builder6.where_expr());
	println!("   -> Username matches 'admin' (any case)");
	println!("   -> Role must be exactly 'Admin' or 'Manager' (case-sensitive)");
}
