use mongodb::{Client, Database, Collection};
use domain::task::entity::TaskEntity;
use domain::task::repository::TaskEntityRepository;

pub struct TaskRepositoryImpl {
    pub client: Client,
    pub database: Database,
    pub collection: Collection<TaskEntity>,
}

impl TaskRepositoryImpl {
    pub fn new(client: Client) -> Self {
        let database = client.database("workflow");
        let collection = database.collection("tasks");
        Self { client, database, collection }
    }
}

impl TaskEntityRepository for TaskRepositoryImpl {
    fn create_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, Error> {
        self.collection.insert_one(task_entity, None).await?;
        Ok(task_entity)
    }

    fn get_task_entity(&self, id: String) -> Result<TaskEntity, Error> {
        self.collection.find_one(doc! {"_id": id}, None).await?;
        Ok(task_entity)
    }

    fn update_task_entity(&self, task_entity: TaskEntity) -> Result<TaskEntity, Error> {
        self.collection.update_one(doc! {"_id": task_entity.id}, doc! {"$set": task_entity}, None).await?;
        Ok(task_entity)
    }

    fn delete_task_entity(&self, id: String) -> Result<(), Error> {
        self.collection.delete_one(doc! {"_id": id}, None).await?;
        Ok(())
    }
}