use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormValueType {
    String,
    Number,
    Bool,
    Json,
    Variable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FormValue {
    Bool(bool),
    Number(f64),
    String(String),
    Json(JsonValue),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Form {
    pub key: String,
    pub value: FormValue,
    #[serde(rename = "type")]
    pub value_type: FormValueType,
    pub description: Option<String>,
}
