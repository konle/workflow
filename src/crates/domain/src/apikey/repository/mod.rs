use async_trait::async_trait;

use crate::apikey::entity::ApiKeyEntity;
use std::error::Error;

pub type RepositoryError = Box<dyn Error + Send + Sync>;

#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn create(&self, entity: &ApiKeyEntity) -> Result<ApiKeyEntity, RepositoryError>;
    async fn get_by_id(&self, tenant_id: &str, id: &str) -> Result<ApiKeyEntity, RepositoryError>;
    async fn get_by_prefix(&self, key_prefix: &str) -> Result<ApiKeyEntity, RepositoryError>;
    async fn update(&self, entity: &ApiKeyEntity) -> Result<ApiKeyEntity, RepositoryError>;
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<ApiKeyEntity>, RepositoryError>;
}
