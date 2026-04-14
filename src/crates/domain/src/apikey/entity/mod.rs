use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::user::entity::TenantRole;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyEntity {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub role: TenantRole,
    pub expires_at: Option<DateTime<Utc>>,
    pub token_ttl_secs: u32,
    pub last_used_at: Option<DateTime<Utc>>,
    pub status: ApiKeyStatus,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
