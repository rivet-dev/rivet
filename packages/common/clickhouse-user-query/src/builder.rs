use clickhouse::query::Query;
use clickhouse::sql::Identifier;
use serde::{Deserialize, Serialize};

use crate::error::{Result, UserQueryError};
use crate::query::{KeyPath, QueryExpr};
use crate::schema::{PropertyType, Schema};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDefinedQueryBuilder {
	where_clause: String,
	group_by_clause: Option<String>,
	bind_values: Vec<BindValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BindValue {
	Bool(bool),
	String(String),
	Number(f64),
}

impl UserDefinedQueryBuilder {
	pub fn new(schema: &Schema, expr: Option<&QueryExpr>) -> Result<Self> {
		let mut builder = QueryBuilder::new(schema);
		let where_clause = match expr {
			Some(e) => builder.build_where_clause(e)?,
			None => "true".to_string(),
		};

		Ok(Self {
			where_clause,
			group_by_clause: None,
			bind_values: builder.bind_values,
		})
	}

	pub fn new_with_group_by(
		schema: &Schema,
		expr: Option<&QueryExpr>,
		group_by: Option<&KeyPath>,
	) -> Result<Self> {
		let mut builder = QueryBuilder::new(schema);
		let where_clause = match expr {
			Some(e) => builder.build_where_clause(e)?,
			None => "true".to_string(),
		};

		let group_by_clause = if let Some(key_path) = group_by {
			let validated_column = builder.validate_group_by_key_path(key_path)?;
			Some(validated_column)
		} else {
			None
		};

		Ok(Self {
			where_clause,
			group_by_clause,
			bind_values: builder.bind_values,
		})
	}

	pub fn bind_to(&self, mut query: Query) -> Query {
		for bind_value in &self.bind_values {
			query = match bind_value {
				BindValue::Bool(v) => query.bind(*v),
				BindValue::String(v) => query.bind(v),
				BindValue::Number(v) => query.bind(*v),
			};
		}
		query
	}

	pub fn where_expr(&self) -> &str {
		&self.where_clause
	}

	pub fn group_by_expr(&self) -> Option<&str> {
		self.group_by_clause.as_deref()
	}
}

struct QueryBuilder<'a> {
	schema: &'a Schema,
	bind_values: Vec<BindValue>,
}

impl<'a> QueryBuilder<'a> {
	fn new(schema: &'a Schema) -> Self {
		Self {
			schema,
			bind_values: Vec::new(),
		}
	}

	fn build_where_clause(&mut self, expr: &QueryExpr) -> Result<String> {
		match expr {
			QueryExpr::And { exprs } => {
				if exprs.is_empty() {
					return Err(UserQueryError::EmptyQuery);
				}
				let clauses: Result<Vec<_>> =
					exprs.iter().map(|e| self.build_where_clause(e)).collect();
				Ok(format!("({})", clauses?.join(" AND ")))
			}
			QueryExpr::Or { exprs } => {
				if exprs.is_empty() {
					return Err(UserQueryError::EmptyQuery);
				}
				let clauses: Result<Vec<_>> =
					exprs.iter().map(|e| self.build_where_clause(e)).collect();
				Ok(format!("({})", clauses?.join(" OR ")))
			}
			QueryExpr::BoolEqual {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Bool)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Bool(*value));
				Ok(format!("{} = ?", column))
			}
			QueryExpr::BoolNotEqual {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Bool)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Bool(*value));
				Ok(format!("{} != ?", column))
			}
			QueryExpr::StringEqual {
				property,
				map_key,
				value,
				case_insensitive,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::String)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::String(value.clone()));
				if *case_insensitive {
					Ok(format!("LOWER({}) = LOWER(?)", column))
				} else {
					Ok(format!("{} = ?", column))
				}
			}
			QueryExpr::StringNotEqual {
				property,
				map_key,
				value,
				case_insensitive,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::String)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::String(value.clone()));
				if *case_insensitive {
					Ok(format!("LOWER({}) != LOWER(?)", column))
				} else {
					Ok(format!("{} != ?", column))
				}
			}
			QueryExpr::StringIn {
				property,
				map_key,
				values,
				case_insensitive,
			} => {
				if values.is_empty() {
					return Err(UserQueryError::EmptyArrayValues("StringIn".to_string()));
				}
				self.validate_property_access(property, map_key, &PropertyType::String)?;
				let column = self.build_column_reference(property, map_key)?;
				let placeholders = vec!["?"; values.len()].join(", ");
				for value in values {
					self.bind_values.push(BindValue::String(value.clone()));
				}
				if *case_insensitive {
					let lower_placeholders = vec!["LOWER(?)"; values.len()].join(", ");
					Ok(format!("LOWER({}) IN ({})", column, lower_placeholders))
				} else {
					Ok(format!("{} IN ({})", column, placeholders))
				}
			}
			QueryExpr::StringNotIn {
				property,
				map_key,
				values,
				case_insensitive,
			} => {
				if values.is_empty() {
					return Err(UserQueryError::EmptyArrayValues("StringNotIn".to_string()));
				}
				self.validate_property_access(property, map_key, &PropertyType::String)?;
				let column = self.build_column_reference(property, map_key)?;
				let placeholders = vec!["?"; values.len()].join(", ");
				for value in values {
					self.bind_values.push(BindValue::String(value.clone()));
				}
				if *case_insensitive {
					let lower_placeholders = vec!["LOWER(?)"; values.len()].join(", ");
					Ok(format!("LOWER({}) NOT IN ({})", column, lower_placeholders))
				} else {
					Ok(format!("{} NOT IN ({})", column, placeholders))
				}
			}
			QueryExpr::StringContains {
				property,
				map_key,
				value,
				case_insensitive,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::String)?;
				let column = self.build_column_reference(property, map_key)?;
				// In ClickHouse, we'll use the LIKE operator with % wildcards
				// We need to escape special LIKE characters in the value
				let escaped_value = value
					.replace("\\", "\\\\")
					.replace("%", "\\%")
					.replace("_", "\\_");
				let like_pattern = format!("%{}%", escaped_value);
				self.bind_values.push(BindValue::String(like_pattern));
				if *case_insensitive {
					// Use ILIKE for case-insensitive matching in ClickHouse
					Ok(format!("{} ILIKE ?", column))
				} else {
					Ok(format!("{} LIKE ?", column))
				}
			}
			QueryExpr::NumberEqual {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Number(*value));
				Ok(format!("{} = ?", column))
			}
			QueryExpr::NumberNotEqual {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Number(*value));
				Ok(format!("{} != ?", column))
			}
			QueryExpr::NumberIn {
				property,
				map_key,
				values,
			} => {
				if values.is_empty() {
					return Err(UserQueryError::EmptyArrayValues("NumberIn".to_string()));
				}
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				let placeholders = vec!["?"; values.len()].join(", ");
				for value in values {
					self.bind_values.push(BindValue::Number(*value));
				}
				Ok(format!("{} IN ({})", column, placeholders))
			}
			QueryExpr::NumberNotIn {
				property,
				map_key,
				values,
			} => {
				if values.is_empty() {
					return Err(UserQueryError::EmptyArrayValues("NumberNotIn".to_string()));
				}
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				let placeholders = vec!["?"; values.len()].join(", ");
				for value in values {
					self.bind_values.push(BindValue::Number(*value));
				}
				Ok(format!("{} NOT IN ({})", column, placeholders))
			}
			QueryExpr::NumberLess {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Number(*value));
				Ok(format!("{} < ?", column))
			}
			QueryExpr::NumberLessOrEqual {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Number(*value));
				Ok(format!("{} <= ?", column))
			}
			QueryExpr::NumberGreater {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Number(*value));
				Ok(format!("{} > ?", column))
			}
			QueryExpr::NumberGreaterOrEqual {
				property,
				map_key,
				value,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::Number)?;
				let column = self.build_column_reference(property, map_key)?;
				self.bind_values.push(BindValue::Number(*value));
				Ok(format!("{} >= ?", column))
			}
			QueryExpr::StringMatchRegex {
				property,
				map_key,
				pattern,
				case_insensitive,
			} => {
				self.validate_property_access(property, map_key, &PropertyType::String)?;
				let column = self.build_column_reference(property, map_key)?;
				if *case_insensitive {
					// For case-insensitive regex, prepend (?i) to the pattern
					let case_insensitive_pattern = format!("(?i){}", pattern);
					self.bind_values
						.push(BindValue::String(case_insensitive_pattern));
					Ok(format!("match({}, ?)", column))
				} else {
					self.bind_values.push(BindValue::String(pattern.clone()));
					Ok(format!("match({}, ?)", column))
				}
			}
		}
	}

	fn validate_property_access(
		&self,
		property: &str,
		map_key: &Option<String>,
		expected_type: &PropertyType,
	) -> Result<()> {
		let prop = self
			.schema
			.get_property(property)
			.ok_or_else(|| UserQueryError::PropertyNotFound(property.to_string()))?;

		if map_key.is_some() && !prop.is_map {
			return Err(UserQueryError::MapKeysNotSupported(property.to_string()));
		}

		if &prop.ty != expected_type {
			return Err(UserQueryError::PropertyTypeMismatch(
				property.to_string(),
				expected_type.type_name().to_string(),
				prop.ty.type_name().to_string(),
			));
		}

		Ok(())
	}

	fn build_column_reference(&self, property: &str, map_key: &Option<String>) -> Result<String> {
		let property_ident = Identifier(property);

		match map_key {
			Some(key) => {
				// Validate map_key name for safe charset
				Self::validate_map_key_name(key)?;

				// For ClickHouse Map access, use string literal syntax
				Ok(format!(
					"{}[{}]",
					property_ident.0,
					format!("'{}'", key.replace("'", "\\'"))
				))
			}
			None => Ok(property_ident.0.to_string()),
		}
	}

	/// Validates that a map_key name only contains safe characters for database queries
	fn validate_map_key_name(name: &str) -> Result<()> {
		// Check if empty
		if name.is_empty() {
			return Err(UserQueryError::InvalidMapKeyName(name.to_string()));
		}

		// Check length (reasonable limit for database identifiers)
		if name.len() > 64 {
			return Err(UserQueryError::InvalidMapKeyName(name.to_string()));
		}

		// Only allow alphanumeric characters and underscores (SQL-safe)
		if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
			return Err(UserQueryError::InvalidMapKeyName(name.to_string()));
		}

		// Must not start with a number (SQL identifier rule)
		if name.chars().next().unwrap().is_numeric() {
			return Err(UserQueryError::InvalidMapKeyName(name.to_string()));
		}

		Ok(())
	}

	fn validate_group_by_key_path(&self, key_path: &KeyPath) -> Result<String> {
		let prop = self
			.schema
			.get_property(&key_path.property)
			.ok_or_else(|| UserQueryError::PropertyNotFound(key_path.property.to_string()))?;

		// Check if property can be grouped by
		if !prop.can_group_by {
			return Err(UserQueryError::PropertyCannotBeGroupedBy(
				key_path.property.to_string(),
			));
		}

		// If there's a map key, the property must be a map
		if key_path.map_key.is_some() && !prop.is_map {
			return Err(UserQueryError::MapKeysNotSupported(
				key_path.property.to_string(),
			));
		}

		// If the property is a map but no key is provided, it cannot be used in GROUP BY
		if prop.is_map && key_path.map_key.is_none() {
			return Err(UserQueryError::MapPropertyCannotBeGroupedBy(
				key_path.property.to_string(),
			));
		}

		// Build the column reference for GROUP BY
		self.build_column_reference(&key_path.property, &key_path.map_key)
	}
}
