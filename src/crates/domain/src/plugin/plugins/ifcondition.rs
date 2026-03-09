use async_trait::async_trait;
use rhai::{Engine, Scope};
use serde_json::Value as JsonValue;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;

use crate::workflow::entity::{
    WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};
use crate::task::entity::TaskTemplate;

pub struct IfConditionPlugin {}

impl IfConditionPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for IfConditionPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::IfCondition(t) => t,
            _ => return Err(anyhow::anyhow!("Invalid task template for IfConditionPlugin")),
        };

        // Initialize Rhai Engine
        let engine = Engine::new();
        let mut scope = Scope::new();

        // 1. Prepare context variables for Rhai scope
        // First try to merge node_instance.context and workflow_instance.context
        // Simplest approach for Rhai is to pass them as `rhai::Dynamic` or stringified JSON, 
        // but Rhai has limited native JSON object traversal without a custom type/plugin.
        // For basic usage, we can stringify and let the user parse it, or map basic key-values.
        // A robust way is to convert `serde_json::Value` to `rhai::Map`.
        
        if let Some(wf_ctx_map) = workflow_instance.context.as_object() {
            for (k, v) in wf_ctx_map {
                scope.push(k, convert_json_to_rhai(v));
            }
        }
        
        if let Some(node_ctx_map) = node_instance.context.as_object() {
            for (k, v) in node_ctx_map {
                scope.push(k, convert_json_to_rhai(v));
            }
        }

        // 2. Evaluate the condition
        let condition_expr = &template.condition;
        let result: bool = engine.eval_with_scope(&mut scope, condition_expr)
            .map_err(|e| anyhow::anyhow!("Failed to evaluate IfCondition: {}", e))?;

        // 3. Determine next node
        let next_node = if result {
            template.then_task.clone()
        } else {
            template.else_task.clone()
        };

        Ok(ExecutionResult::success(next_node))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::IfCondition
    }
}

// Helper to convert serde_json::Value to rhai::Dynamic
fn convert_json_to_rhai(val: &JsonValue) -> rhai::Dynamic {
    match val {
        JsonValue::Null => rhai::Dynamic::UNIT,
        JsonValue::Bool(b) => rhai::Dynamic::from(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from(f)
            } else {
                rhai::Dynamic::UNIT
            }
        },
        JsonValue::String(s) => rhai::Dynamic::from(s.clone()),
        JsonValue::Array(arr) => {
            let mut rhai_arr = rhai::Array::new();
            for item in arr {
                rhai_arr.push(convert_json_to_rhai(item));
            }
            rhai::Dynamic::from_array(rhai_arr)
        },
        JsonValue::Object(obj) => {
            let mut rhai_map = rhai::Map::new();
            for (k, v) in obj {
                rhai_map.insert(k.clone().into(), convert_json_to_rhai(v));
            }
            rhai::Dynamic::from_map(rhai_map)
        },
    }
}