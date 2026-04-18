use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::task::entity::task_definition::ApprovalMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Approve,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub user_id: String,
    pub decision: Decision,
    pub comment: Option<String>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalInstanceEntity {
    pub id: String,
    pub tenant_id: String,
    pub workflow_instance_id: String,
    pub node_id: String,
    pub title: String,
    pub description: Option<String>,
    pub approval_mode: ApprovalMode,
    pub approvers: Vec<String>,
    pub decisions: Vec<ApprovalDecision>,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub applicant_id: Option<String>,
}
