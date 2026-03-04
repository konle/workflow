use async_trait::async_trait;

use crate::plugin::interface::{ExecutionResult, PluginExecutor, PluginInterface};
use crate::shared::workflow::TaskType;
use crate::shared::job::{ExecuteTaskJob, WorkflowCallerContext};
use crate::workflow::entity::{
    WorkflowInstanceEntity, WorkflowNodeInstanceEntity,
};

pub struct HttpPlugin {}

impl HttpPlugin {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PluginInterface for HttpPlugin {
    async fn execute(
        &self,
        _executor: &dyn PluginExecutor,
        node_instance: &mut WorkflowNodeInstanceEntity,
        workflow_instance: &mut WorkflowInstanceEntity,
    ) -> anyhow::Result<ExecutionResult> {
        // 构造异步任务
        let job = ExecuteTaskJob {
            task_instance_id: format!("{}-{}", workflow_instance.workflow_instance_id, node_instance.node_id),
            tenant_id: "default".to_string(), // TODO: 从上下文中获取
            caller_context: Some(WorkflowCallerContext {
                workflow_instance_id: workflow_instance.workflow_instance_id.clone(),
                node_id: node_instance.node_id.clone(),
            }),
        };

        // 返回 AsyncDispatch，让 Manager 挂起工作流并投递任务
        Ok(ExecutionResult::async_dispatch(job))
    }

    fn plugin_type(&self) -> TaskType {
        TaskType::Http
    }
}
