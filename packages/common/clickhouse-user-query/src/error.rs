use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserQueryError {
    #[error("Property '{0}' not found in schema")]
    PropertyNotFound(String),
    
    #[error("Property '{0}' does not support subproperties")]
    SubpropertiesNotSupported(String),
    
    #[error("Property '{0}' type mismatch: expected {1}, got {2}")]
    PropertyTypeMismatch(String, String, String),
    
    #[error("Invalid property or subproperty name '{0}': must contain only alphanumeric characters and underscores")]
    InvalidPropertyName(String),
    
    #[error("Empty query expression")]
    EmptyQuery,
    
    #[error("Empty array values in {0} operation")]
    EmptyArrayValues(String),
}

pub type Result<T> = std::result::Result<T, UserQueryError>;