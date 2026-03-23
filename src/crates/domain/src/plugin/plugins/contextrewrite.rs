use async_trait::async_trait;
use rhai::Scope;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::plugin::rhai_engine;
use crate::shared::workflow::TaskType;
use crate::task::entity::{MergeMode, TaskTemplate};
use crate::workflow::entity::{WorkflowInstanceEntity, WorkflowNodeInstanceEntity};

pub struct ContextRewritePlugin {}

impl ContextRewritePlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for ContextRewritePlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::ContextRewrite(t) => t,
            _ => return Err(anyhow::anyhow!("Invalid task template for ContextRewritePlugin")),
        };

        let engine = rhai_engine::create_engine();
        let ast = rhai_engine::compile_script(&engine, &template.script)?;

        let mut scope = Scope::new();
        rhai_engine::inject_context(&mut scope, &node_instance.context);

        let result = engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast)
            .map_err(|e| anyhow::anyhow!("ContextRewrite script error: {}", e))?;

        let result_map = rhai_engine::rhai_map_to_json(result)?;

        match template.merge_mode {
            MergeMode::Merge => {
                if let Some(ctx_obj) = workflow_instance.context.as_object_mut() {
                    for (k, v) in result_map {
                        ctx_obj.insert(k, v);
                    }
                } else {
                    workflow_instance.context = serde_json::Value::Object(result_map);
                }
            }
            MergeMode::Replace => {
                workflow_instance.context = serde_json::Value::Object(result_map);
            }
        }

        node_instance.task_instance.output = Some(serde_json::json!({
            "rewritten_keys": workflow_instance.context.as_object()
                .map(|o| o.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
        }));

        Ok(ExecutionResult::success(None))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::ContextRewrite
    }
}
