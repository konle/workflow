use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub init: InitConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub mongo_url: String,
    pub redis_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub variable_encrypt_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InitConfig {
    pub admin_username: String,
    pub admin_password: String,
    pub admin_email: String,
    pub default_tenant_name: String,
    pub default_tenant_description: String,
}

impl AppConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(Path::new(path))
            .map_err(|e| anyhow::anyhow!("failed to read config file '{}': {}", path, e))?;
        let mut config: AppConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse config file '{}': {}", path, e))?;

        if let Ok(v) = std::env::var("MONGO_URL") {
            config.database.mongo_url = v;
        }
        if let Ok(v) = std::env::var("REDIS_URL") {
            config.database.redis_url = v;
        }
        if let Ok(v) = std::env::var("API_PORT") {
            if let Ok(port) = v.parse() {
                config.server.port = port;
            }
        }
        if let Ok(v) = std::env::var("VARIABLE_ENCRYPT_KEY") {
            config.security.variable_encrypt_key = v;
        }

        Ok(config)
    }
}
