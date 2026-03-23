use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::{Client, Database, Collection};
use domain::shared::workflow::TaskInstanceStatus;
use domain::task::entity::{TaskEntity, TaskInstanceEntity};
use domain::task::repository::{TaskEntityRepository, TaskInstanceEntityRepository, RepositoryError};

pub struct TaskRepositoryImpl {
    pub client: Client,
    pub database: Database,
    pub collection: Collection<TaskEntity>,
}

pub struct TaskInstanceRepositoryImpl {
    pub client: Client,
    pub database: Database,
    pub collection: Collection<TaskInstanceEntity>,
}

impl TaskRepositoryImpl {
    pub fn new(client: Client) -> Self {
        let database = client.database("workflow");
        let collection = database.collection("tasks");
        Self { client, database, collection }
    }
}

impl TaskInstanceRepositoryImpl {
    pub fn new(client: Client) -> Self {
        let database = client.database("workflow");
        let collection = database.collection("task_instances");
        Self { client, database, collection }
    }
}

#[async_trait]
impl TaskInstanceEntityRepository for TaskInstanceRepositoryImpl {
    async fn create_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError> {
        self.collection.insert_one(&task_instance_entity).await?;
        Ok(task_instance_entity)
    }

    async fn get_task_instance_entity(&self, id: String) -> Result<TaskInstanceEntity, RepositoryError> {
        let task_instance_entity = self.collection
            .find_one(doc! {"task_instance_id": &id})
            .await?
            .ok_or_else(|| format!("task instance entity not found: {}", id))?;
        Ok(task_instance_entity)
    }

    async fn get_task_instance_entity_scoped(&self, tenant_id: &str, id: &str) -> Result<TaskInstanceEntity, RepositoryError> {
        let entity = self.collection
            .find_one(doc! {"tenant_id": tenant_id, "task_instance_id": id})
            .await?
            .ok_or_else(|| format!("task instance not found: {} in tenant {}", id, tenant_id))?;
        Ok(entity)
    }

    async fn list_task_instance_entities(&self, tenant_id: &str) -> Result<Vec<TaskInstanceEntity>, RepositoryError> {
        let cursor = self.collection
            .find(doc! {"tenant_id": tenant_id})
            .await?;
        let results: Vec<TaskInstanceEntity> = cursor.try_collect().await?;
        Ok(results)
    }

    async fn update_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError> {
        let filter = doc! {"task_instance_id": &task_instance_entity.task_instance_id};
        self.collection.replace_one(filter, &task_instance_entity).await?;
        Ok(task_instance_entity)
    }

    async fn transfer_status(
        &self,
        task_instance_id: &str,
        from_status: &TaskInstanceStatus,
        to_status: &TaskInstanceStatus,
    ) -> Result<TaskInstanceEntity, RepositoryError> {
        let from_str = format!("{:?}", from_status);
        let to_str = format!("{:?}", to_status);

        let filter = doc! {
            "task_instance_id": task_instance_id,
            "task_status": &from_str,
        };
        let update = doc! {
            "$set": {
                "task_status": &to_str,
                "updated_at": chrono::Utc::now().to_rfc3339(),
            }
        };

        let result = self.collection
            .find_one_and_update(filter, update)
            .return_document(mongodb::options::ReturnDocument::After)
            .await?
            .ok_or_else(|| format!(
                "CAS failed: task instance {} not in expected state {}",
                task_instance_id, from_str
            ))?;

        Ok(result)
    }
}

#[async_trait]
impl TaskEntityRepository for TaskRepositoryImpl {
    async fn create_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError> {
        self.collection.insert_one(&task_entity).await?;
        Ok(task_entity)
    }

    async fn get_task_entity(&self, id: String) -> Result<TaskEntity, RepositoryError> {
        let task_entity = self.collection
            .find_one(doc! {"id": &id})
            .await?
            .ok_or_else(|| format!("task entity not found: {}", id))?;
        Ok(task_entity)
    }

    async fn get_task_entity_scoped(&self, tenant_id: &str, id: &str) -> Result<TaskEntity, RepositoryError> {
        let entity = self.collection
            .find_one(doc! {"tenant_id": tenant_id, "id": id})
            .await?
            .ok_or_else(|| format!("task entity not found: {} in tenant {}", id, tenant_id))?;
        Ok(entity)
    }

    async fn list_task_entities(&self, tenant_id: &str) -> Result<Vec<TaskEntity>, RepositoryError> {
        let cursor = self.collection
            .find(doc! {"tenant_id": tenant_id})
            .await?;
        let results: Vec<TaskEntity> = cursor.try_collect().await?;
        Ok(results)
    }

    async fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError> {
        let filter = doc! {"tenant_id": &task_entity.tenant_id, "id": &task_entity.id};
        self.collection.replace_one(filter, &task_entity).await?;
        Ok(task_entity)
    }

    async fn delete_task_entity(&self, tenant_id: &str, id: &str) -> Result<(), RepositoryError> {
        self.collection.delete_one(doc! {"tenant_id": tenant_id, "id": id}).await?;
        Ok(())
    }
}
