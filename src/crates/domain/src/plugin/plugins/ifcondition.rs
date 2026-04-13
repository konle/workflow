use async_trait::async_trait;
use rhai::Scope;
use tracing::{debug, error};

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::plugin::rhai_engine;
use crate::shared::workflow::TaskType;
use crate::task::entity::task_definition::TaskTemplate;
use crate::workflow::entity::workflow_definition::{WorkflowInstanceEntity, WorkflowNodeInstanceEntity};

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
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for IfConditionPlugin");
                return Err(anyhow::anyhow!("Invalid task template for IfConditionPlugin"));
            }
        };

        let engine = rhai_engine::create_engine();
        let mut scope = Scope::new();

        // `node_instance.context` is resolve_variables + `nodes` (see `run_node`); single source of truth.
        rhai_engine::inject_context_flat(&mut scope, &node_instance.context);

        let result: bool = engine.eval_with_scope(&mut scope, &template.condition)
            .map_err(|e| {
                error!(
                    workflow_instance_id = %workflow_instance.workflow_instance_id,
                    node_id = %node_instance.node_id,
                    condition = %template.condition,
                    error = %e,
                    "failed to evaluate IfCondition"
                );
                anyhow::anyhow!("Failed to evaluate IfCondition: {}", e)
            })?;

        let next_node = if result {
            template.then_task.clone()
        } else {
            template.else_task.clone()
        };
        let out_data = serde_json::json!({
            "if_condition_result": result,
            "next_node": next_node,
            "then_task": template.then_task.clone(),
            "else_task": template.else_task.clone(),
            "condition": template.condition.clone(),
        });
        node_instance.task_instance.input = Some(serde_json::json!({
            "condition": template.condition.clone(),
            "name": template.name.clone(),
        }));
        node_instance.task_instance.output = Some(out_data);

        debug!(
            node_id = %node_instance.node_id,
            condition = %template.condition,
            result = result,
            next_node = ?next_node,
            "IfCondition evaluated"
        );

        Ok(ExecutionResult::success(next_node))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::IfCondition
    }
}
