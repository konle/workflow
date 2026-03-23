use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::shared::workflow::TaskType;
use crate::task::entity::{ParallelMode, TaskTemplate};
use crate::workflow::entity::{
    NodeExecutionStatus, WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct ForkJoinPlugin {}

impl ForkJoinPlugin {
    pub fn new() -> Self {
        Self {}
    }

    fn resolve_task_key_by_child_id<'a>(
        template: &'a crate::task::entity::ForkJoinTemplate,
        child_task_id: &str,
    ) -> Option<&'a str> {
        let suffix = child_task_id.rsplit('-').next()?;
        let index: usize = suffix.parse().ok()?;
        template.tasks.get(index).map(|item| item.task_key.as_str())
    }
}

#[async_trait]
impl PluginInterface for ForkJoinPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::ForkJoin(t) => t,
            _ => return Err(anyhow::anyhow!("Invalid task template for ForkJoinPlugin")),
        };

        if template.tasks.is_empty() {
            return Ok(ExecutionResult::success(None));
        }

        let total_tasks = template.tasks.len();
        let concurrency = template.concurrency as usize;
        let initial_dispatch = std::cmp::min(total_tasks, concurrency);

        let mut results_map = serde_json::Map::new();
        for item in &template.tasks {
            results_map.insert(item.task_key.clone(), JsonValue::Null);
        }

        let state = serde_json::json!({
            "total_tasks": total_tasks,
            "dispatched_count": initial_dispatch,
            "success_count": 0,
            "failed_count": 0,
            "results": results_map,
        });
        node_instance.task_instance.output = Some(state);

        let mut jobs = Vec::with_capacity(initial_dispatch);
        for index in 0..initial_dispatch {
            let caller_context = WorkflowCallerContext {
                workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                node_id: node_instance.node_id.clone(),
                parent_task_instance_id: Some(node_instance.task_instance.id.clone()),
                item_index: Some(index),
            };

            let job = ExecuteTaskJob {
                task_instance_id: format!(
                    "{}-{}-{}",
                    workflow_instance.workflow_instance_id,
                    node_instance.node_id,
                    index
                ),
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
        child_task_id: &str,
        status: &NodeExecutionStatus,
        output: &Option<serde_json::Value>,
        error_message: &Option<String>,
        _input: &Option<serde_json::Value>,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::ForkJoin(t) => t.clone(),
            _ => return Err(anyhow::anyhow!("Invalid task template for ForkJoinPlugin")),
        };

        let mut state = node_instance
            .task_instance
            .output
            .clone()
            .unwrap_or(serde_json::json!({}));

        let mut success_count = state["success_count"].as_u64().unwrap_or(0);
        let mut failed_count = state["failed_count"].as_u64().unwrap_or(0);
        let total_tasks = state["total_tasks"].as_u64().unwrap_or(0);
        let mut dispatched_count = state["dispatched_count"].as_u64().unwrap_or(0);

        if *status == NodeExecutionStatus::Success {
            success_count += 1;
        } else if *status == NodeExecutionStatus::Failed {
            failed_count += 1;
        }

        if let Some(task_key) = Self::resolve_task_key_by_child_id(&template, child_task_id) {
            let result_entry = serde_json::json!({
                "status": format!("{:?}", status),
                "output": output,
                "error": error_message,
            });
            if let Some(results) = state.get_mut("results").and_then(|r| r.as_object_mut()) {
                results.insert(task_key.to_string(), result_entry);
            }
        }

        let concurrency = template.concurrency as u64;
        let mode = template.mode.clone();

        let has_failed_threshold = match template.max_failures {
            Some(max) => failed_count > max as u64,
            None => false,
        };

        let exec_result = if has_failed_threshold {
            node_instance.error_message = Some(format!(
                "ForkJoin max_failures threshold exceeded ({} failed)",
                failed_count
            ));
            ExecutionResult::failed()
        } else if success_count + failed_count == total_tasks {
            ExecutionResult::success(None)
        } else {
            let mut indices_to_dispatch = Vec::new();

            if mode == ParallelMode::Rolling {
                if dispatched_count < total_tasks {
                    indices_to_dispatch.push(dispatched_count);
                    dispatched_count += 1;
                }
            } else if mode == ParallelMode::Batch {
                if success_count + failed_count == dispatched_count {
                    let end = std::cmp::min(dispatched_count + concurrency, total_tasks);
                    for i in dispatched_count..end {
                        indices_to_dispatch.push(i);
                    }
                    dispatched_count = end;
                }
            }

            let new_jobs: Vec<ExecuteTaskJob> = indices_to_dispatch
                .into_iter()
                .map(|idx| {
                    let caller_context = WorkflowCallerContext {
                        workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                        node_id: node_instance.node_id.clone(),
                        parent_task_instance_id: Some(node_instance.task_instance.id.clone()),
                        item_index: Some(idx as usize),
                    };
                    ExecuteTaskJob {
                        task_instance_id: format!(
                            "{}-{}-{}",
                            workflow_instance.workflow_instance_id,
                            node_instance.node_id,
                            idx
                        ),
                        tenant_id: workflow_instance.tenant_id.clone(),
                        caller_context: Some(caller_context),
                    }
                })
                .collect();

            ExecutionResult::async_dispatch_multiple(new_jobs)
        };

        state["success_count"] = serde_json::json!(success_count);
        state["failed_count"] = serde_json::json!(failed_count);
        state["dispatched_count"] = serde_json::json!(dispatched_count);
        node_instance.task_instance.output = Some(state);

        Ok(exec_result)
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::ForkJoin
    }
}
