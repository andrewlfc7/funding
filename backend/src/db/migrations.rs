use ::clickhouse::Client;
use anyhow::Result;

pub async fn create_pool() -> Result<Client> {
    crate::db::clickhouse::create_client_from_env().await
}

pub async fn create_pool_with_url(_db_url: &str) -> Result<Client> {
    create_pool().await
}
