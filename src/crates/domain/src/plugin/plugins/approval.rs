use async_trait::async_trait;
use tracing::{info, error};

use crate::approval::entity::ApprovalStatus;
use crate::approval::service::ApprovalService;
use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::task::entity::TaskTemplate;
use crate::workflow::entity::workflow_definition::{
    NodeExecutionStatus, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct ApprovalPlugin {
    approval_svc: ApprovalService,
}

impl ApprovalPlugin {
    pub fn new(approval_svc: ApprovalService) -> Self {
        Self { approval_svc }
    }
}

#[async_trait]
impl PluginInterface for ApprovalPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::Approval(t) => t,
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for ApprovalPlugin");
                return Err(anyhow::anyhow!("Invalid task template for ApprovalPlugin"));
            }
        };

        node_instance.task_instance.input = Some(serde_json::json!({
            "title": template.title.clone(),
            "name": template.name.clone(),
            "description": template.description.clone(),
        }));

        let approval = self
            .approval_svc
            .create_approval(
                &workflow_instance.tenant_id,
                &workflow_instance.workflow_instance_id,
                &node_instance.node_id,
                template,
                &node_instance.context,
            )
            .await
            .map_err(|e| {
                error!(
                    workflow_instance_id = %workflow_instance.workflow_instance_id,
                    node_id = %node_instance.node_id,
                    error = %e,
                    "failed to create approval instance"
                );
                anyhow::anyhow!("Failed to create approval instance: {}", e)
            })?;

        info!(
            approval_id = %approval.id,
            workflow_instance_id = %workflow_instance.workflow_instance_id,
            node_id = %node_instance.node_id,
            "approval created, workflow suspended"
        );

        node_instance.task_instance.output = Some(serde_json::json!({
            "approval_id": approval.id,
            "approvers": approval.approvers,
            "approval_mode": format!("{:?}", approval.approval_mode),
        }));

        Ok(ExecutionResult::suspended())
    }

    async fn handle_callback(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        _workflow_instance: &mut WorkflowInstanceEntity,
        _child_task_id: &str,
        status: &NodeExecutionStatus,
        output: &Option<serde_json::Value>,
        error_message: &Option<String>,
        _input: &Option<serde_json::Value>,
    ) -> anyhow::Result<ExecutionResult> {
        node_instance.error_message = error_message.clone();
        node_instance.task_instance.input = _input.clone();
        node_instance.task_instance.output = output.clone();
        node_instance.task_instance.error_message = error_message.clone();

        match status {
            NodeExecutionStatus::Success => Ok(ExecutionResult::success(None)),
            NodeExecutionStatus::Skipped => Ok(ExecutionResult::skipped(None)),
            NodeExecutionStatus::Failed => Ok(ExecutionResult::failed()),
            _ => Ok(ExecutionResult::pending()),
        }
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Approval
    }
}

pub fn approval_status_to_node_status(status: &ApprovalStatus) -> NodeExecutionStatus {
    match status {
        ApprovalStatus::Approved => NodeExecutionStatus::Success,
        ApprovalStatus::Rejected => NodeExecutionStatus::Failed,
        ApprovalStatus::Pending => NodeExecutionStatus::Suspended,
    }
}
