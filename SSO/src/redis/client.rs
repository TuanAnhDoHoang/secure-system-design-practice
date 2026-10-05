use anyhow::Context;
use redis::aio::ConnectionManager;

use crate::config::RedisConfig;

/// Khởi tạo kết nối Redis ConnectionManager hỗ trợ pooling và tự động kết nối lại
pub async fn init_redis(config: &RedisConfig) -> anyhow::Result<ConnectionManager> {
    let client = redis::Client::open(config.url.as_str())
        .context(format!("Invalid Redis URL: {}", config.url))?;
    let manager = ConnectionManager::new(client)
        .await
        .context("Failed to establish Redis ConnectionManager")?;
    Ok(manager)
}
