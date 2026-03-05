use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::workflow::entity::{WorkflowInstanceEntity, WorkflowNodeInstanceEntity};
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
            _ => return Err(anyhow::anyhow!("Invalid task template for ParallelPlugin")),
        };

        let items_path = &template.items_path;
        
        // Basic json pointer lookup
        let pointer_path = if items_path.starts_with('/') {
            items_path.clone()
        } else {
            format!("/{}", items_path.replace(".", "/"))
        };

        let items_val = workflow_instance.context.pointer(&pointer_path)
            .or_else(|| node_instance.context.pointer(&pointer_path));

        let items = match items_val {
            Some(JsonValue::Array(arr)) => arr,
            _ => return Err(anyhow::anyhow!("Items path '{}' did not resolve to a JSON array", items_path)),
        };

        if items.is_empty() {
            return Ok(ExecutionResult::success(None));
        }

        let total_items = items.len();
        let concurrency = template.concurrency as usize;
        let initial_dispatch_count = std::cmp::min(total_items, concurrency);

        // Initialize state in parent task output
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
                // Generate a unique task instance ID for the sub-task
                task_instance_id: format!("{}-{}-{}", workflow_instance.workflow_instance_id, node_instance.node_id, index),
                tenant_id: "default".to_string(), // TODO: 从上下文中获取
                caller_context: Some(caller_context),
            };
            jobs.push(job);
        }

        Ok(ExecutionResult::async_dispatch_multiple(jobs))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Parallel
    }
}