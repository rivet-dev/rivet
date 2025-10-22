use crate::error::{Result, UserQueryError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
	pub properties: Vec<Property>,
}

impl Schema {
	pub fn new(properties: Vec<Property>) -> Result<Self> {
		// All property validation happens in Property::new()
		Ok(Self { properties })
	}

	pub fn get_property(&self, name: &str) -> Option<&Property> {
		self.properties.iter().find(|p| p.name == name)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
	pub name: String,
	pub is_map: bool,
	/// The type of values in the property. For map properties, this is the type of the map values.
	pub ty: PropertyType,
	/// Whether this property can be used in GROUP BY clauses
	pub can_group_by: bool,
}

impl Property {
	pub fn new(name: String, is_map: bool, ty: PropertyType) -> Result<Self> {
		validate_property_name(&name)?;
		Ok(Self {
			name,
			is_map,
			ty,
			can_group_by: false,
		})
	}

	pub fn with_group_by(mut self, can_group_by: bool) -> Self {
		self.can_group_by = can_group_by;
		self
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyType {
	Bool,
	String,
	Number,
}

impl PropertyType {
	pub fn type_name(&self) -> &'static str {
		match self {
			PropertyType::Bool => "bool",
			PropertyType::String => "string",
			PropertyType::Number => "number",
		}
	}
}

fn validate_property_name(name: &str) -> Result<()> {
	if name.is_empty() {
		return Err(UserQueryError::InvalidPropertyName(name.to_string()));
	}

	if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
		return Err(UserQueryError::InvalidPropertyName(name.to_string()));
	}

	if name.chars().next().unwrap().is_numeric() {
		return Err(UserQueryError::InvalidPropertyName(name.to_string()));
	}

	Ok(())
}
