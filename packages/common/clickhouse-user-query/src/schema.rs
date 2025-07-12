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
    pub supports_subproperties: bool,
    pub ty: PropertyType,
}

impl Property {
    pub fn new(name: String, supports_subproperties: bool, ty: PropertyType) -> Result<Self> {
        validate_property_name(&name)?;
        Ok(Self {
            name,
            supports_subproperties,
            ty,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyType {
    Bool,
    String,
    Number,
    ArrayString,
}

impl PropertyType {
    pub fn type_name(&self) -> &'static str {
        match self {
            PropertyType::Bool => "bool",
            PropertyType::String => "string",
            PropertyType::Number => "number",
            PropertyType::ArrayString => "array[string]",
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

