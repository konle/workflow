//! Apply [`ExecutionResult`] to the workflow graph: node status, instance status, persistence, queue dispatch.

use super::loop_action::LoopAction;
use super::PluginManager;
use crate::plugin::interface::ExecutionResult;
use crate::shared::workflow::WorkflowInstanceStatus;
use crate::workflow::entity::{NodeExecutionStatus, WorkflowInstanceEntity};
use tracing::error;

impl PluginManager {
    pub(super) async fn apply_exec_result(
        &self,
        instance: &mut WorkflowInstanceEntity,
        node_index: usize,
        exec_result: ExecutionResult,
    ) -> anyhow::Result<LoopAction> {
        instance.nodes[node_index].status = exec_result.status.clone();
        let action = match exec_result.status {
            NodeExecutionStatus::Success | NodeExecutionStatus::Skipped => {
                if let Some(jump_to_node) = exec_result.jump_to_node {
                    instance.current_node = jump_to_node.clone();
                    instance.nodes[node_index].next_node = Some(jump_to_node);
                    LoopAction::Advance
                } else if let Some(next) = instance.nodes[node_index].next_node.clone() {
                    instance.current_node = next;
                    LoopAction::Advance
                } else {
                    self.workflow_instance_svc
                        .complete_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Completed;
                    LoopAction::Done
                }
            }
            NodeExecutionStatus::Failed => {
                self.workflow_instance_svc
                    .fail_instance(&instance.workflow_instance_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                instance.status = WorkflowInstanceStatus::Failed;
                LoopAction::Done
            }
            NodeExecutionStatus::Pending | NodeExecutionStatus::Suspended => {
                if !exec_result.dispatch_jobs.is_empty() || !exec_result.dispatch_workflow_jobs.is_empty()
                {
                    self.workflow_instance_svc
                        .await_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Await;
                } else {
                    self.workflow_instance_svc
                        .suspend_instance(&instance.workflow_instance_id)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    instance.status = WorkflowInstanceStatus::Suspended;
                }
                LoopAction::Done
            }
            _ => LoopAction::Retry,
        };

        self.save_instance_and_bump_epoch(instance).await?;

        for job in &exec_result.dispatch_jobs {
            self.ensure_task_instance_for_job(instance, node_index, job).await?;
        }

        for job in exec_result.dispatch_jobs {
            if let Err(e) = self.dispatcher.dispatch_task(job.clone()).await {
                error!(
                    task_instance_id = %job.task_instance_id,
                    error = %e,
                    "failed to dispatch task"
                );
                return Err(e.into());
            }
        }
        for job in exec_result.dispatch_workflow_jobs {
            if let Err(e) = self.dispatcher.dispatch_workflow(job.clone()).await {
                error!(
                    workflow_instance_id = %job.workflow_instance_id,
                    error = %e,
                    "failed to dispatch workflow"
                );
                return Err(e.into());
            }
        }

        Ok(action)
    }

    pub(super) async fn save_instance_and_bump_epoch(
        &self,
        instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<()> {
        self.workflow_instance_svc
            .save_workflow_instance(instance)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        instance.epoch += 1;
        instance.updated_at = chrono::Utc::now();
        Ok(())
    }
}
