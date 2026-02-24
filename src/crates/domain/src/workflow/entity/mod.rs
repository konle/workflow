use crate::shared::form::Form;
use crate::shared::workflow::{TaskType, WorkflowInstanceStatus};
use crate::shared::workflow::WorkflowStatus;
use crate::task::entity::TaskTemplate;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowMetaEntity {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub status: WorkflowStatus,
    pub form: Vec<Form>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEntity {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub version: u32,
    pub workflow_meta_id: String,
    pub status: WorkflowStatus,
    pub nodes: Vec<WorkflowNodeEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNodeEntity {
    pub node_id: String,
    pub node_type: TaskType,
    pub config: TaskTemplate,
    pub context: JsonValue,
    pub next_node: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowInstanceEntity {
    pub workflow_instance_id: String,
    pub workflow_meta_id: String,
    pub workflow_version: u32,
    pub status: WorkflowInstanceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub context: JsonValue,
    pub nodes: Vec<WorkflowNodeInstanceEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
    Suspended,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeOutput {
    pub data: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNodeInstanceEntity {
    pub node_id: String,
    pub node_type: TaskType,
    pub config: TaskTemplate,
    pub context: JsonValue,
    pub next_node: Option<String>,
    pub status: NodeExecutionStatus,
    pub output: Option<NodeOutput>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
