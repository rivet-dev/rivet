use serde::{Deserialize, Serialize};

/// Represents a path to a property, optionally including a map key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyPath {
	pub property: String,
	pub map_key: Option<String>,
}

impl KeyPath {
	pub fn new(property: String) -> Self {
		Self {
			property,
			map_key: None,
		}
	}

	pub fn with_map_key(property: String, map_key: String) -> Self {
		Self {
			property,
			map_key: Some(map_key),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryExpr {
	And {
		exprs: Vec<QueryExpr>,
	},
	Or {
		exprs: Vec<QueryExpr>,
	},
	BoolEqual {
		property: String,
		map_key: Option<String>,
		value: bool,
	},
	BoolNotEqual {
		property: String,
		map_key: Option<String>,
		value: bool,
	},
	StringEqual {
		property: String,
		map_key: Option<String>,
		value: String,
		case_sensitive: bool,
	},
	StringNotEqual {
		property: String,
		map_key: Option<String>,
		value: String,
		case_sensitive: bool,
	},
	StringIn {
		property: String,
		map_key: Option<String>,
		values: Vec<String>,
		case_sensitive: bool,
	},
	StringNotIn {
		property: String,
		map_key: Option<String>,
		values: Vec<String>,
		case_sensitive: bool,
	},
	StringMatchRegex {
		property: String,
		map_key: Option<String>,
		pattern: String,
		case_sensitive: bool,
	},
	NumberEqual {
		property: String,
		map_key: Option<String>,
		value: f64,
	},
	NumberNotEqual {
		property: String,
		map_key: Option<String>,
		value: f64,
	},
	NumberIn {
		property: String,
		map_key: Option<String>,
		values: Vec<f64>,
	},
	NumberNotIn {
		property: String,
		map_key: Option<String>,
		values: Vec<f64>,
	},
	NumberLess {
		property: String,
		map_key: Option<String>,
		value: f64,
	},
	NumberLessOrEqual {
		property: String,
		map_key: Option<String>,
		value: f64,
	},
	NumberGreater {
		property: String,
		map_key: Option<String>,
		value: f64,
	},
	NumberGreaterOrEqual {
		property: String,
		map_key: Option<String>,
		value: f64,
	},
}
