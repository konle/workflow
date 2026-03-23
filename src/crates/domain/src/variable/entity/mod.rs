use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableScope {
    Tenant,
    WorkflowMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Number,
    Bool,
    Json,
    Secret,
}

impl VariableType {
    pub fn is_secret(&self) -> bool {
        matches!(self, VariableType::Secret)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableEntity {
    pub id: String,
    pub tenant_id: String,
    pub scope: VariableScope,
    pub scope_id: String,
    pub key: String,
    pub value: String,
    pub variable_type: VariableType,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
