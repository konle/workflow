use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{Client, Database, Collection};
use mongodb::bson::doc;
use domain::shared::workflow::WorkflowInstanceStatus;
use domain::workflow::entity::{WorkflowEntity, WorkflowInstanceEntity, WorkflowMetaEntity};
use domain::workflow::repository::{WorkflowDefinitionRepository, WorkflowInstanceRepository, RepositoryError};

pub struct WorkflowDefinitionRepositoryImpl {
    pub client: Client,
    pub database: Database,
    pub collection: Collection<WorkflowEntity>,
    pub workflow_meta_collection: Collection<WorkflowMetaEntity>,
}

impl WorkflowDefinitionRepositoryImpl {
    pub fn new(client: Client) -> Self {
        let database = client.database("workflow");
        let collection = database.collection("workflow_entities");
        let workflow_meta_collection = database.collection("workflow_meta_entities");
        Self { client, database, collection, workflow_meta_collection }
    }
}

#[async_trait]
impl WorkflowDefinitionRepository for WorkflowDefinitionRepositoryImpl {
    async fn get_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<WorkflowEntity, RepositoryError> {
        let workflow_entity = self.collection
            .find_one(doc! {"workflow_meta_id": &workflow_meta_id, "version": &version})
            .await?
            .ok_or_else(|| format!("workflow entity not found: {} version: {}", workflow_meta_id, version))?;
        Ok(workflow_entity)
    }

    async fn save_workflow_entity(&self, entity: &WorkflowEntity) -> Result<(), RepositoryError> {
        self.collection.insert_one(entity).await?;
        Ok(())
    }

    async fn delete_workflow_entity(&self, workflow_meta_id: String, version: u32) -> Result<(), RepositoryError> {
        self.collection.delete_one(doc! {"workflow_meta_id": &workflow_meta_id, "version": &version}).await?;
        Ok(())
    }

    async fn get_workflow_meta_entity(&self, workflow_meta_id: String) -> Result<WorkflowMetaEntity, RepositoryError> {
        let workflow_meta_entity = self.workflow_meta_collection
            .find_one(doc! {"workflow_meta_id": &workflow_meta_id})
            .await?
            .ok_or_else(|| format!("workflow meta entity not found: {}", &workflow_meta_id))?;
        Ok(workflow_meta_entity)
    }

    async fn save_workflow_meta_entity(&self, entity: &WorkflowMetaEntity) -> Result<(), RepositoryError> {
        self.workflow_meta_collection.insert_one(entity).await?;
        Ok(())
    }

    async fn get_workflow_meta_entity_scoped(&self, tenant_id: &str, workflow_meta_id: &str) -> Result<WorkflowMetaEntity, RepositoryError> {
        let entity = self.workflow_meta_collection
            .find_one(doc! {"tenant_id": tenant_id, "workflow_meta_id": workflow_meta_id})
            .await?
            .ok_or_else(|| format!("workflow meta entity not found: {} in tenant {}", workflow_meta_id, tenant_id))?;
        Ok(entity)
    }

    async fn list_workflow_meta_entities(&self, tenant_id: &str) -> Result<Vec<WorkflowMetaEntity>, RepositoryError> {
        let cursor = self.workflow_meta_collection
            .find(doc! {"tenant_id": tenant_id})
            .await?;
        let results: Vec<WorkflowMetaEntity> = cursor.try_collect().await?;
        Ok(results)
    }

    async fn delete_workflow_meta_entity(&self, tenant_id: &str, workflow_meta_id: &str) -> Result<(), RepositoryError> {
        self.workflow_meta_collection
            .delete_one(doc! {"tenant_id": tenant_id, "workflow_meta_id": workflow_meta_id})
            .await?;
        Ok(())
    }

    async fn create_workflow_meta_entity(&self, workflow_meta_entity: &WorkflowMetaEntity) -> Result<WorkflowMetaEntity, RepositoryError> {
        self.workflow_meta_collection.insert_one(workflow_meta_entity).await?;
        Ok(workflow_meta_entity.clone())
    }
}

pub struct WorkflowInstanceRepositoryImpl {
    pub client: Client,
    pub database: Database,
    pub workflow_instance_collection: Collection<WorkflowInstanceEntity>,
}

impl WorkflowInstanceRepositoryImpl {
    pub fn new(client: Client) -> Self {
        let database = client.database("workflow");
        let workflow_instance_collection = database.collection("workflow_instances");
        Self { client, database, workflow_instance_collection }
    }
}

#[async_trait]
impl WorkflowInstanceRepository for WorkflowInstanceRepositoryImpl {
    async fn get_workflow_instance(&self, id: String) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let workflow_instance = self.workflow_instance_collection
            .find_one(doc! {"workflow_instance_id": &id})
            .await?
            .ok_or_else(|| format!("workflow instance not found: {}", id))?;
        Ok(workflow_instance)
    }

    async fn get_workflow_instance_scoped(&self, tenant_id: &str, id: &str) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let instance = self.workflow_instance_collection
            .find_one(doc! {"tenant_id": tenant_id, "workflow_instance_id": id})
            .await?
            .ok_or_else(|| format!("workflow instance not found: {} in tenant {}", id, tenant_id))?;
        Ok(instance)
    }

    async fn list_workflow_instances(&self, tenant_id: &str) -> Result<Vec<WorkflowInstanceEntity>, RepositoryError> {
        let cursor = self.workflow_instance_collection
            .find(doc! {"tenant_id": tenant_id})
            .await?;
        let results: Vec<WorkflowInstanceEntity> = cursor.try_collect().await?;
        Ok(results)
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

    async fn acquire_lock(
        &self,
        workflow_instance_id: &str,
        worker_id: &str,
        duration_ms: u64,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        let now = chrono::Utc::now();
        let expiration = now - chrono::Duration::milliseconds(duration_ms as i64); 
        
        let filter = doc! {
            "workflow_instance_id": workflow_instance_id,
            "$or": [
                { "locked_by": mongodb::bson::Bson::Null },
                { "locked_at": { "$lt": expiration.to_rfc3339() } }
            ]
        };

        let update_doc = doc! {
            "$set": {
                "locked_by": worker_id,
                "locked_duration": duration_ms as i64,
                "locked_at": now.to_rfc3339(),
                "updated_at": now.to_rfc3339(),
            },
            "$inc": { "epoch": 1 }
        };

        let result = self.workflow_instance_collection
            .find_one_and_update(filter, update_doc)
            .return_document(mongodb::options::ReturnDocument::After)
            .await?
            .ok_or_else(|| format!("failed to acquire lock for instance {}", workflow_instance_id))?;

        Ok(result)
    }

    async fn release_lock(
        &self,
        workflow_instance_id: &str,
        worker_id: &str,
    ) -> Result<(), RepositoryError> {
        let filter = doc! {
            "workflow_instance_id": workflow_instance_id,
            "locked_by": worker_id,
        };

        let update_doc = doc! {
            "$set": {
                "locked_by": mongodb::bson::Bson::Null,
                "locked_duration": mongodb::bson::Bson::Null,
                "locked_at": mongodb::bson::Bson::Null,
                "updated_at": chrono::Utc::now().to_rfc3339(),
            },
            "$inc": { "epoch": 1 }
        };

        let result = self.workflow_instance_collection
            .update_one(filter, update_doc)
            .await?;

        if result.matched_count == 0 {
            return Err(format!("failed to release lock for instance {} (not held by {})", workflow_instance_id, worker_id).into());
        }

        Ok(())
    }

    async fn create_workflow_instance(
        &self,
        instance: &WorkflowInstanceEntity,
    ) -> Result<WorkflowInstanceEntity, RepositoryError> {
        self.workflow_instance_collection.insert_one(instance).await?;
        Ok(instance.clone())
    }

    async fn save_workflow_instance(
        &self,
        instance: &WorkflowInstanceEntity,
    ) -> Result<(), RepositoryError> {
        let current_epoch = instance.epoch as i64;
        let filter = doc! {
            "workflow_instance_id": &instance.workflow_instance_id,
            "epoch": current_epoch, // CAS check
        };
        
        let mut update_instance = instance.clone();
        update_instance.epoch += 1;
        update_instance.updated_at = chrono::Utc::now();

        let update_doc = mongodb::bson::to_document(&update_instance)
            .map_err(|e| format!("Failed to serialize instance: {}", e))?;

        let update = doc! {
            "$set": update_doc
        };

        let result = self.workflow_instance_collection
            .update_one(filter.clone(), update)
            .await?;

        if result.matched_count == 0 {
            // Check if it exists at all (might be an insert or a CAS failure)
            let exists = self.workflow_instance_collection
                .count_documents(doc! { "workflow_instance_id": &instance.workflow_instance_id })
                .await?;
                
            if exists == 0 {
                // If it doesn't exist, it's an initial insert, which is valid for save()
                self.workflow_instance_collection
                    .insert_one(update_instance)
                    .await?;
                return Ok(());
            }

            return Err(format!(
                "Optimistic lock failed for workflow {}: expected epoch {}",
                instance.workflow_instance_id, current_epoch
            ).into());
        }

        Ok(())
    }
}
