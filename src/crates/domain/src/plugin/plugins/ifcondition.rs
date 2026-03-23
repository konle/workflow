use async_trait::async_trait;
use rhai::Scope;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::plugin::rhai_engine;
use crate::shared::workflow::TaskType;
use crate::task::entity::TaskTemplate;
use crate::workflow::entity::{WorkflowInstanceEntity, WorkflowNodeInstanceEntity};

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

        let engine = rhai_engine::create_engine();
        let mut scope = Scope::new();

        rhai_engine::inject_context_flat(&mut scope, &workflow_instance.context);
        rhai_engine::inject_context_flat(&mut scope, &node_instance.context);

        let result: bool = engine.eval_with_scope(&mut scope, &template.condition)
            .map_err(|e| anyhow::anyhow!("Failed to evaluate IfCondition: {}", e))?;

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
