use async_trait::async_trait;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::job::{ExecuteWorkflowJob, WorkflowCallerContext, WorkflowEvent};
use crate::shared::workflow::TaskType;
use crate::task::entity::TaskTemplate;
use crate::workflow::entity::{WorkflowInstanceEntity, WorkflowNodeInstanceEntity};
use crate::workflow::service::{WorkflowDefinitionService, WorkflowInstanceService};

const MAX_SUB_WORKFLOW_DEPTH: u32 = 10;

pub struct SubWorkflowPlugin {
    definition_svc: WorkflowDefinitionService,
    instance_svc: WorkflowInstanceService,
}

impl SubWorkflowPlugin {
    pub fn new(
        definition_svc: WorkflowDefinitionService,
        instance_svc: WorkflowInstanceService,
    ) -> Self {
        Self { definition_svc, instance_svc }
    }
}

#[async_trait]
impl PluginInterface for SubWorkflowPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        let template = match &node_instance.task_instance.task_template {
            TaskTemplate::SubWorkflow(t) => t,
            _ => return Err(anyhow::anyhow!("Invalid task template for SubWorkflowPlugin")),
        };

        let child_depth = workflow_instance.depth + 1;
        if child_depth > MAX_SUB_WORKFLOW_DEPTH {
            return Err(anyhow::anyhow!(
                "Sub-workflow nesting depth exceeded maximum ({}), possible circular reference",
                MAX_SUB_WORKFLOW_DEPTH
            ));
        }

        let workflow_entity = self.definition_svc
            .get_workflow_entity(template.workflow_meta_id.clone(), template.workflow_version)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load sub-workflow template: {}", e))?;

        let child_context = template.input_mapping
            .clone()
            .unwrap_or(workflow_instance.context.clone());

        let parent_ctx = WorkflowCallerContext {
            workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
            node_id: node_instance.node_id.clone(),
            parent_task_instance_id: None,
            item_index: None,
        };

        let child_instance = self.instance_svc
            .create_instance(&workflow_instance.tenant_id, &workflow_entity, child_context, Some(parent_ctx), child_depth)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create sub-workflow instance: {}", e))?;

        node_instance.task_instance.output = Some(serde_json::json!({
            "child_workflow_instance_id": child_instance.workflow_instance_id,
        }));

        let job = ExecuteWorkflowJob {
            workflow_instance_id: child_instance.workflow_instance_id,
            tenant_id: workflow_instance.tenant_id.clone(),
            event: WorkflowEvent::Start,
        };

        Ok(ExecutionResult::async_dispatch_workflow(job))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::SubWorkflow
    }
}
