use std::collections::HashMap;
use std::sync::Arc;
use crate::shared::workflow::TaskType;
use crate::task::interface::{TaskExecutionResult, TaskExecutor};
use crate::task::entity::TaskInstanceEntity;
use crate::task::service::TaskInstanceService;

pub struct TaskManager {
    executors: HashMap<TaskType, Box<dyn TaskExecutor>>,
    task_instance_svc: Arc<TaskInstanceService>,
}

impl TaskManager {
    pub fn new(task_instance_svc: Arc<TaskInstanceService>) -> Self {
        Self {
            executors: HashMap::new(),
            task_instance_svc,
        }
    }

    pub fn task_instance_svc(&self) -> &TaskInstanceService {
        &self.task_instance_svc
    }

    pub fn register(&mut self, executor: Box<dyn TaskExecutor>) {
        let task_type = executor.task_type();
        self.executors.insert(task_type, executor);
    }

    pub async fn execute_task(
        &self,
        task_instance: &TaskInstanceEntity,
    ) -> anyhow::Result<TaskExecutionResult> {
        let executor = self
            .executors
            .get(&task_instance.task_type)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no task executor registered for task type: {:?}",
                    task_instance.task_type
                )
            })?;

        executor.execute_task(task_instance).await
    }
}