use async_trait::async_trait;
use mongodb::{Client, Database, Collection};
use mongodb::bson::doc;
use domain::shared::workflow::WorkflowInstanceStatus;
use domain::workflow::entity::{WorkflowEntity, WorkflowInstanceEntity};
use domain::workflow::repository::{WorkflowEntityRepository, RepositoryError};

pub struct WorkflowRepositoryImpl {
    pub client: Client,
    pub database: Database,
    pub collection: Collection<WorkflowEntity>,
    pub workflow_instance_collection: Collection<WorkflowInstanceEntity>,
}

impl WorkflowRepositoryImpl {
    pub fn new(client: Client) -> Self {
        let database = client.database("workflow");
        let collection = database.collection("workflow_entities");
        let workflow_instance_collection = database.collection("workflow_instances");
        Self { client, database, collection, workflow_instance_collection }
    }
}

#[async_trait]
impl WorkflowEntityRepository for WorkflowRepositoryImpl {
    async fn get_workflow_entity(&self, id: String) -> Result<WorkflowEntity, RepositoryError> {
        let workflow_entity = self.collection
            .find_one(doc! {"_id": &id})
            .await?
            .ok_or_else(|| format!("workflow entity not found: {}", id))?;
        Ok(workflow_entity)
    }

    async fn get_workflow_instance(&self, id: String) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let workflow_instance = self.workflow_instance_collection
            .find_one(doc! {"workflow_instance_id": &id})
            .await?
            .ok_or_else(|| format!("workflow instance not found: {}", id))?;
        Ok(workflow_instance)
    }

    async fn transfer_status(
        &self,
        workflow_instance_id: &str,
        from_status: &WorkflowInstanceStatus,
        to_status: &WorkflowInstanceStatus,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let from_str = format!("{:?}", from_status);
        let to_str = format!("{:?}", to_status);

        let filter = doc! {
            "workflow_instance_id": workflow_instance_id,
            "status": &from_str,
        };
        let update = doc! {
            "$set": {
                "status": &to_str,
                "updated_at": chrono::Utc::now().to_rfc3339(),
            }
        };

        let result = self.workflow_instance_collection
            .find_one_and_update(filter, update)
            .return_document(mongodb::options::ReturnDocument::After)
            .await?
            .ok_or_else(|| format!(
                "CAS failed: instance {} not in expected state {}",
                workflow_instance_id, from_str
            ))?;

        Ok(result)
    }

    async fn save_workflow_instance(
        &self,
        instance: &WorkflowInstanceEntity,
    ) -> Result<(), RepositoryError> {
        let filter = doc! {
            "workflow_instance_id": &instance.workflow_instance_id,
        };
        self.workflow_instance_collection
            .replace_one(filter, instance)
            .upsert(true)
            .await?;
        Ok(())
    }
}
