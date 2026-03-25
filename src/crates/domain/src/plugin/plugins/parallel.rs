use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tracing::{debug, warn, error};

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::workflow::entity::{
    NodeExecutionStatus, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};
use crate::task::entity::TaskTemplate;

pub struct ParallelPlugin {}

impl ParallelPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for ParallelPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::Parallel(t) => t,
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for ParallelPlugin");
                return Err(anyhow::anyhow!("Invalid task template for ParallelPlugin"));
            }
        };

        let items_path = &template.items_path;
        
        let pointer_path = if items_path.starts_with('/') {
            items_path.clone()
        } else {
            format!("/{}", items_path.replace(".", "/"))
        };

        let items_val = workflow_instance.context.pointer(&pointer_path)
            .or_else(|| node_instance.context.pointer(&pointer_path));

        let items = match items_val {
            Some(JsonValue::Array(arr)) => arr,
            _ => {
                error!(
                    node_id = %node_instance.node_id,
                    items_path = %items_path,
                    "items path did not resolve to a JSON array"
                );
                return Err(anyhow::anyhow!("Items path '{}' did not resolve to a JSON array", items_path));
            }
        };

        if items.is_empty() {
            debug!(node_id = %node_instance.node_id, "parallel: empty items, completing immediately");
            return Ok(ExecutionResult::success(None));
        }

        let total_items = items.len();
        let concurrency = template.concurrency as usize;
        let initial_dispatch_count = std::cmp::min(total_items, concurrency);

        debug!(
            node_id = %node_instance.node_id,
            total_items = total_items,
            concurrency = concurrency,
            initial_dispatch = initial_dispatch_count,
            "parallel: dispatching initial batch"
        );

        let state = serde_json::json!({
            "total_items": total_items,
            "dispatched_count": initial_dispatch_count,
            "success_count": 0,
            "failed_count": 0
        });
        node_instance.task_instance.output = Some(state);

        let mut jobs = Vec::new();
        for index in 0..initial_dispatch_count {
            let caller_context = WorkflowCallerContext {
                workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                node_id: node_instance.node_id.clone(),
                parent_task_instance_id: Some(node_instance.task_instance.id.clone()),
                item_index: Some(index),
            };

            let job = ExecuteTaskJob {
                task_instance_id: format!("{}-{}-{}", workflow_instance.workflow_instance_id, node_instance.node_id, index),
                tenant_id: workflow_instance.tenant_id.clone(),
                caller_context: Some(caller_context),
            };
            jobs.push(job);
        }

        Ok(ExecutionResult::async_dispatch_multiple(jobs))
    }

    async fn handle_callback(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
        _child_task_id: &str,
        status: &NodeExecutionStatus,
        _output: &Option<serde_json::Value>,
        _error_message: &Option<String>,
        _input: &Option<serde_json::Value>,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::Parallel(t) => t,
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for ParallelPlugin callback");
                return Err(anyhow::anyhow!("Invalid task template for ParallelPlugin"));
            }
        };

        let mut state = node_instance.task_instance.output.clone().unwrap_or(serde_json::json!({}));
        let mut success_count = state["success_count"].as_u64().unwrap_or(0);
        let mut failed_count = state["failed_count"].as_u64().unwrap_or(0);
        let total_items = state["total_items"].as_u64().unwrap_or(0);
        let mut dispatched_count = state["dispatched_count"].as_u64().unwrap_or(0);

        if *status == NodeExecutionStatus::Success {
            success_count += 1;
        } else if *status == NodeExecutionStatus::Failed {
            failed_count += 1;
        }

        let concurrency = template.concurrency as u64;
        let mode = template.mode.clone();
        let max_failures = template.max_failures;

        let has_failed_threshold = match max_failures {
            Some(max) => failed_count > max as u64,
            None => false,
        };

        let exec_result = if has_failed_threshold {
            warn!(
                node_id = %node_instance.node_id,
                failed_count = failed_count,
                max_failures = ?max_failures,
                "parallel: max_failures threshold exceeded"
            );
            node_instance.error_message = Some(format!("Parallel max_failures threshold exceeded ({} failed)", failed_count));
            ExecutionResult::failed()
        } else if success_count + failed_count == total_items {
            debug!(
                node_id = %node_instance.node_id,
                success_count = success_count,
                failed_count = failed_count,
                "parallel: all items completed"
            );
            ExecutionResult::success(None)
        } else {
            let mut jobs_to_dispatch = Vec::new();
            
            if mode == crate::task::entity::ParallelMode::Rolling {
                if dispatched_count < total_items {
                    jobs_to_dispatch.push(dispatched_count);
                    dispatched_count += 1;
                }
            } else if mode == crate::task::entity::ParallelMode::Batch {
                if success_count + failed_count == dispatched_count {
                    let end = std::cmp::min(dispatched_count + concurrency, total_items);
                    for i in dispatched_count..end {
                        jobs_to_dispatch.push(i);
                    }
                    dispatched_count = end;
                }
            }

            let mut new_jobs = Vec::new();
            for idx in jobs_to_dispatch {
                let caller_context = WorkflowCallerContext {
                    workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                    node_id: node_instance.node_id.clone(),
                    parent_task_instance_id: Some(node_instance.task_instance.id.clone()),
                    item_index: Some(idx as usize),
                };
                let child_job = ExecuteTaskJob {
                    task_instance_id: format!("{}-{}-{}", workflow_instance.workflow_instance_id, node_instance.node_id, idx),
                    tenant_id: workflow_instance.tenant_id.clone(),
                    caller_context: Some(caller_context),
                };
                new_jobs.push(child_job);
            }
            ExecutionResult::async_dispatch_multiple(new_jobs)
        };

        state["success_count"] = serde_json::json!(success_count);
        state["failed_count"] = serde_json::json!(failed_count);
        state["dispatched_count"] = serde_json::json!(dispatched_count);
        node_instance.task_instance.output = Some(state);

        Ok(exec_result)
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Parallel
    }
}
