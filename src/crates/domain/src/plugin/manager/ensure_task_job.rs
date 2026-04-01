//! Materialize `task_instances` rows for async jobs (Parallel / ForkJoin children use inner template + type).

use super::PluginManager;
use crate::shared::job::ExecuteTaskJob;
use crate::shared::workflow::TaskInstanceStatus;
use crate::task::entity::{TaskInstanceEntity, TaskTemplate};
use crate::workflow::entity::WorkflowInstanceEntity;

impl PluginManager {
    pub(super) async fn ensure_task_instance_for_job(
        &self,
        instance: &WorkflowInstanceEntity,
        node_index: usize,
        job: &ExecuteTaskJob,
    ) -> anyhow::Result<()> {
        let Some(task_svc) = &self.task_instance_svc else {
            return Ok(());
        };

        if task_svc
            .get_task_instance_entity(job.task_instance_id.clone())
            .await
            .is_ok()
        {
            return Ok(());
        }

        let now = chrono::Utc::now();
        let parent = &instance.nodes[node_index].task_instance;
        let (child_template, child_task_type) = match &parent.task_template {
            TaskTemplate::Parallel(pt) => {
                let inner = (*pt.task_template).clone();
                let tt = inner.task_type();
                (inner, tt)
            }
            TaskTemplate::ForkJoin(fj) => {
                let idx = job
                    .caller_context
                    .as_ref()
                    .and_then(|c| c.item_index)
                    .ok_or_else(|| {
                        anyhow::anyhow!("ForkJoin dispatch job missing item_index in caller_context")
                    })?;
                let item = fj.tasks.get(idx).ok_or_else(|| {
                    anyhow::anyhow!(
                        "ForkJoin item_index {} out of range (len {})",
                        idx,
                        fj.tasks.len()
                    )
                })?;
                let inner = item.task_template.clone();
                let tt = inner.task_type();
                (inner, tt)
            }
            _ => (parent.task_template.clone(), parent.task_type.clone()),
        };

        let mut task_instance: TaskInstanceEntity = parent.clone();
        task_instance.task_template = child_template;
        task_instance.task_type = child_task_type;
        task_instance.id = job.task_instance_id.clone();
        task_instance.task_id = parent.task_id.clone();
        task_instance.task_instance_id = job.task_instance_id.clone();
        task_instance.tenant_id = job.tenant_id.clone();
        task_instance.caller_context = job.caller_context.clone();
        task_instance.created_at = now;
        task_instance.updated_at = now;
        task_instance.input = None;
        task_instance.output = None;
        task_instance.error_message = None;
        task_instance.execution_duration = None;
        task_instance.task_status = TaskInstanceStatus::Pending;

        task_svc
            .create_task_instance_entity(task_instance)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}
