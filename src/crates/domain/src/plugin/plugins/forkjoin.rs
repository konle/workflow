use async_trait::async_trait;
use serde_json::Value as JsonValue;
use tracing::{debug, warn, error};

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::shared::workflow::TaskType;
use crate::task::entity::{ParallelMode, TaskTemplate};
use crate::workflow::entity::workflow_definition::{
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
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for ForkJoinPlugin");
                return Err(anyhow::anyhow!("Invalid task template for ForkJoinPlugin"));
            }
        };

        if template.tasks.is_empty() {
            debug!(node_id = %node_instance.node_id, "forkjoin: empty tasks, completing immediately");
            return Ok(ExecutionResult::success(None));
        }

        let total_tasks = template.tasks.len();
        let concurrency = template.concurrency as usize;
        let initial_dispatch = std::cmp::min(total_tasks, concurrency);

        debug!(
            node_id = %node_instance.node_id,
            total_tasks = total_tasks,
            concurrency = concurrency,
            initial_dispatch = initial_dispatch,
            "forkjoin: dispatching initial batch"
        );

        let mut results_map = serde_json::Map::new();
        for item in &template.tasks {
            results_map.insert(item.task_key.clone(), JsonValue::Null);
        }

        node_instance.task_instance.input = Some(serde_json::json!({
            "task_keys": template.tasks.iter().map(|t| t.task_key.clone()).collect::<Vec<_>>(),
            "concurrency": template.concurrency,
            "mode": format!("{:?}", template.mode),
            "max_failures": template.max_failures,
        }));

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
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for ForkJoinPlugin callback");
                return Err(anyhow::anyhow!("Invalid task template for ForkJoinPlugin"));
            }
        };

        let mut state = node_instance
            .task_instance
            .output
            .clone()
            .unwrap_or(serde_json::json!({}));

        let processed: Vec<String> = state
            .get("processed_callbacks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if processed.contains(&child_task_id.to_string()) {
            warn!(
                node_id = %node_instance.node_id,
                child_task_id = %child_task_id,
                "forkjoin: duplicate callback ignored"
            );
            return Ok(ExecutionResult::async_dispatch_multiple(vec![]));
        }

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
            warn!(
                node_id = %node_instance.node_id,
                failed_count = failed_count,
                max_failures = ?template.max_failures,
                "forkjoin: max_failures threshold exceeded"
            );
            node_instance.error_message = Some(format!(
                "ForkJoin max_failures threshold exceeded ({} failed)",
                failed_count
            ));
            ExecutionResult::failed()
        } else if success_count + failed_count == total_tasks {
            debug!(
                node_id = %node_instance.node_id,
                success_count = success_count,
                failed_count = failed_count,
                "forkjoin: all tasks completed"
            );
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
        let mut updated_processed = processed;
        updated_processed.push(child_task_id.to_string());
        state["processed_callbacks"] = serde_json::json!(updated_processed);
        node_instance.task_instance.output = Some(state);

        Ok(exec_result)
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::ForkJoin
    }
}
