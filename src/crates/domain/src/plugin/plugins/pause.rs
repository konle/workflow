use async_trait::async_trait;
use chrono::Utc;
use tracing::info;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::task::entity::task_definition::{PauseMode, TaskTemplate};
use crate::workflow::entity::workflow_definition::{
    WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct PausePlugin;

impl PausePlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PluginInterface for PausePlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::Pause(t) => t.clone(),
            other => {
                return Err(anyhow::anyhow!(
                    "Invalid template for PausePlugin: {:?}",
                    other
                ));
            }
        };

        let resume_at = Utc::now() + chrono::Duration::seconds(template.wait_seconds as i64);

        node_instance.task_instance.output = Some(serde_json::json!({
            "mode": format!("{:?}", template.mode),
            "wait_seconds": template.wait_seconds,
            "resume_at": resume_at.to_rfc3339(),
        }));

        info!(
            workflow_instance_id = %workflow_instance.workflow_instance_id,
            node_id = %node_instance.node_id,
            mode = ?template.mode,
            wait_seconds = template.wait_seconds,
            resume_at = %resume_at.to_rfc3339(),
            "pause node suspended"
        );

        Ok(ExecutionResult::suspended())
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Pause
    }
}
