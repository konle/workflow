use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::{Client, Database, Collection};
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
            .find_one(doc! {"_id": &id})
            .await?
            .ok_or_else(|| format!("task instance entity not found: {}", id))?;
        Ok(task_instance_entity)
    }

    async fn update_task_instance_entity(&self, task_instance_entity: TaskInstanceEntity) -> Result<TaskInstanceEntity, RepositoryError> {
        let filter = doc! {"_id": &task_instance_entity.id};
        self.collection.replace_one(filter, &task_instance_entity).await?;
        Ok(task_instance_entity)
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
            .find_one(doc! {"_id": &id})
            .await?
            .ok_or_else(|| format!("task entity not found: {}", id))?;
        Ok(task_entity)
    }

    async fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, RepositoryError> {
        let filter = doc! {"_id": &task_entity.id};
        self.collection.replace_one(filter, &task_entity).await?;
        Ok(task_entity)
    }

    async fn delete_task_entity(&self, id: String) -> Result<(), RepositoryError> {
        self.collection.delete_one(doc! {"_id": &id}).await?;
        Ok(())
    }
}
