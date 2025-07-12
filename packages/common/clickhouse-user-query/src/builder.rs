use clickhouse::query::Query;
use clickhouse::sql::Identifier;
use serde::{Deserialize, Serialize};

use crate::error::{Result, UserQueryError};
use crate::query::QueryExpr;
use crate::schema::{PropertyType, Schema};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDefinedQueryBuilder {
    where_clause: String,
    bind_values: Vec<BindValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BindValue {
    Bool(bool),
    String(String),
    Number(f64),
    ArrayString(Vec<String>),
}

impl UserDefinedQueryBuilder {
    pub fn new(schema: &Schema, expr: &QueryExpr) -> Result<Self> {
        let mut builder = QueryBuilder::new(schema);
        let where_clause = builder.build_where_clause(expr)?;
        
        if where_clause.trim().is_empty() {
            return Err(UserQueryError::EmptyQuery);
        }
        
        Ok(Self {
            where_clause,
            bind_values: builder.bind_values,
        })
    }
    
    pub fn bind_to(&self, mut query: Query) -> Query {
        for bind_value in &self.bind_values {
            query = match bind_value {
                BindValue::Bool(v) => query.bind(*v),
                BindValue::String(v) => query.bind(v),
                BindValue::Number(v) => query.bind(*v),
                BindValue::ArrayString(v) => query.bind(v),
            };
        }
        query
    }
    
    pub fn where_expr(&self) -> &str {
        &self.where_clause
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
                let clauses: Result<Vec<_>> = exprs
                    .iter()
                    .map(|e| self.build_where_clause(e))
                    .collect();
                Ok(format!("({})", clauses?.join(" AND ")))
            }
            QueryExpr::Or { exprs } => {
                if exprs.is_empty() {
                    return Err(UserQueryError::EmptyQuery);
                }
                let clauses: Result<Vec<_>> = exprs
                    .iter()
                    .map(|e| self.build_where_clause(e))
                    .collect();
                Ok(format!("({})", clauses?.join(" OR ")))
            }
            QueryExpr::BoolEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Bool)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Bool(*value));
                Ok(format!("{} = ?", column))
            }
            QueryExpr::BoolNotEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Bool)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Bool(*value));
                Ok(format!("{} != ?", column))
            }
            QueryExpr::StringEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::String)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::String(value.clone()));
                Ok(format!("{} = ?", column))
            }
            QueryExpr::StringNotEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::String)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::String(value.clone()));
                Ok(format!("{} != ?", column))
            }
            QueryExpr::ArrayContains { property, subproperty, values } => {
                if values.is_empty() {
                    return Err(UserQueryError::EmptyArrayValues("ArrayContains".to_string()));
                }
                self.validate_property_access(property, subproperty, &PropertyType::ArrayString)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::ArrayString(values.clone()));
                Ok(format!("hasAny({}, ?)", column))
            }
            QueryExpr::ArrayDoesNotContain { property, subproperty, values } => {
                if values.is_empty() {
                    return Err(UserQueryError::EmptyArrayValues("ArrayDoesNotContain".to_string()));
                }
                self.validate_property_access(property, subproperty, &PropertyType::ArrayString)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::ArrayString(values.clone()));
                Ok(format!("NOT hasAny({}, ?)", column))
            }
            QueryExpr::NumberEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Number)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Number(*value));
                Ok(format!("{} = ?", column))
            }
            QueryExpr::NumberNotEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Number)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Number(*value));
                Ok(format!("{} != ?", column))
            }
            QueryExpr::NumberLess { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Number)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Number(*value));
                Ok(format!("{} < ?", column))
            }
            QueryExpr::NumberLessOrEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Number)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Number(*value));
                Ok(format!("{} <= ?", column))
            }
            QueryExpr::NumberGreater { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Number)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Number(*value));
                Ok(format!("{} > ?", column))
            }
            QueryExpr::NumberGreaterOrEqual { property, subproperty, value } => {
                self.validate_property_access(property, subproperty, &PropertyType::Number)?;
                let column = self.build_column_reference(property, subproperty)?;
                self.bind_values.push(BindValue::Number(*value));
                Ok(format!("{} >= ?", column))
            }
        }
    }
    
    fn validate_property_access(
        &self,
        property: &str,
        subproperty: &Option<String>,
        expected_type: &PropertyType,
    ) -> Result<()> {
        let prop = self.schema.get_property(property)
            .ok_or_else(|| UserQueryError::PropertyNotFound(property.to_string()))?;
        
        if subproperty.is_some() && !prop.supports_subproperties {
            return Err(UserQueryError::SubpropertiesNotSupported(property.to_string()));
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
    
    fn build_column_reference(&self, property: &str, subproperty: &Option<String>) -> Result<String> {
        let property_ident = Identifier(property);
        
        match subproperty {
            Some(subprop) => {
                // For ClickHouse Map access, use string literal syntax
                Ok(format!("{}[{}]", property_ident.0, format!("'{}'", subprop.replace("'", "\\'"))))
            }
            None => Ok(property_ident.0.to_string()),
        }
    }
}

