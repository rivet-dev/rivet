use serde::{Deserialize, Serialize};

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
        subproperty: Option<String>,
        value: bool,
    },
    BoolNotEqual {
        property: String,
        subproperty: Option<String>,
        value: bool,
    },
    StringEqual {
        property: String,
        subproperty: Option<String>,
        value: String,
    },
    StringNotEqual {
        property: String,
        subproperty: Option<String>,
        value: String,
    },
    ArrayContains {
        property: String,
        subproperty: Option<String>,
        values: Vec<String>,
    },
    ArrayDoesNotContain {
        property: String,
        subproperty: Option<String>,
        values: Vec<String>,
    },
    NumberEqual {
        property: String,
        subproperty: Option<String>,
        value: f64,
    },
    NumberNotEqual {
        property: String,
        subproperty: Option<String>,
        value: f64,
    },
    NumberLess {
        property: String,
        subproperty: Option<String>,
        value: f64,
    },
    NumberLessOrEqual {
        property: String,
        subproperty: Option<String>,
        value: f64,
    },
    NumberGreater {
        property: String,
        subproperty: Option<String>,
        value: f64,
    },
    NumberGreaterOrEqual {
        property: String,
        subproperty: Option<String>,
        value: f64,
    },
}

