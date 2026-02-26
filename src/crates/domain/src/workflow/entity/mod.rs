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
    pub entry_node: String, // 入口节点
    pub current_node: String, // 当前节点
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

impl WorkflowInstanceEntity {
    pub fn get_current_node(&self) -> String {
        if self.current_node.is_empty() {
            self.entry_node.clone()
        } else {
            self.current_node.clone()
        }
    }
    pub fn get_node_by_id(&self, node_id: &str) -> Option<&WorkflowNodeInstanceEntity> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    pub fn is_completed(&self) -> bool {
        self.status == WorkflowInstanceStatus::Completed
    }
    pub fn is_failed(&self) -> bool {
        self.status == WorkflowInstanceStatus::Failed
    }
    pub fn is_suspended(&self) -> bool {
        self.status == WorkflowInstanceStatus::Suspended
    }
    pub fn is_canceled(&self) -> bool {
        self.status == WorkflowInstanceStatus::Canceled
    }
    pub fn is_running(&self) -> bool {
        self.status == WorkflowInstanceStatus::Running
    }
    pub fn is_pending(&self) -> bool {
        self.status == WorkflowInstanceStatus::Pending
    }
}