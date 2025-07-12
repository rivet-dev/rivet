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
		#[serde(default)]
		map_key: Option<String>,
		value: bool,
	},
	BoolNotEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: bool,
	},
	StringEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: String,
		#[serde(default)]
		case_insensitive: bool,
	},
	StringNotEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: String,
		#[serde(default)]
		case_insensitive: bool,
	},
	StringIn {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		values: Vec<String>,
		#[serde(default)]
		case_insensitive: bool,
	},
	StringNotIn {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		values: Vec<String>,
		#[serde(default)]
		case_insensitive: bool,
	},
	StringContains {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: String,
		#[serde(default)]
		case_insensitive: bool,
	},
	StringMatchRegex {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		pattern: String,
		#[serde(default)]
		case_insensitive: bool,
	},
	NumberEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: f64,
	},
	NumberNotEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: f64,
	},
	NumberIn {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		values: Vec<f64>,
	},
	NumberNotIn {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		values: Vec<f64>,
	},
	NumberLess {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: f64,
	},
	NumberLessOrEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: f64,
	},
	NumberGreater {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: f64,
	},
	NumberGreaterOrEqual {
		property: String,
		#[serde(default)]
		map_key: Option<String>,
		value: f64,
	},
}
