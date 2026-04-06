use async_trait::async_trait;
use rhai::Scope;
use tracing::{debug, error};

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::plugin::rhai_engine;
use crate::shared::workflow::TaskType;
use crate::task::entity::{MergeMode, TaskTemplate};
use crate::workflow::entity::workflow_definition::{WorkflowInstanceEntity, WorkflowNodeInstanceEntity};

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
            other => {
                error!(node_id = %node_instance.node_id, template = ?other, "invalid template for ContextRewritePlugin");
                return Err(anyhow::anyhow!("Invalid task template for ContextRewritePlugin"));
            }
        };

        let engine = rhai_engine::create_engine();
        let ast = rhai_engine::compile_script(&engine, &template.script)
            .map_err(|e| {
                error!(
                    workflow_instance_id = %workflow_instance.workflow_instance_id,
                    node_id = %node_instance.node_id,
                    error = %e,
                    "failed to compile ContextRewrite script"
                );
                e
            })?;

        let mut scope = Scope::new();
        rhai_engine::inject_context(&mut scope, &node_instance.context);

        let result = engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast)
            .map_err(|e| {
                error!(
                    workflow_instance_id = %workflow_instance.workflow_instance_id,
                    node_id = %node_instance.node_id,
                    error = %e,
                    "ContextRewrite script execution error"
                );
                anyhow::anyhow!("ContextRewrite script error: {}", e)
            })?;

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

        debug!(
            node_id = %node_instance.node_id,
            merge_mode = ?template.merge_mode,
            "ContextRewrite applied"
        );

        node_instance.task_instance.input = Some(serde_json::json!({
            "name": template.name.clone(),
            "script": template.script.clone(),
            "merge_mode": format!("{:?}", template.merge_mode),
        }));

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
