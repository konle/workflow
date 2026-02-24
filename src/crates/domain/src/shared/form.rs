use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FormValue {
    String(String),
    Number(f64),
    Bool(bool),
    Json(JsonValue),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Form {
    pub key: String,
    pub value: FormValue,
    #[serde(rename = "type")]
    pub form_type: String,
    pub description: Option<String>,
}
