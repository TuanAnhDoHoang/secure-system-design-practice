use std::env;

use anyhow::Context;
use redis::aio::ConnectionManager;

pub async fn init_redis() -> anyhow::Result<ConnectionManager> {
    let redis_url = env::var("REDIS_URL").context("REDIS_URL must be set")?;
    let client = redis::Client::open(redis_url).context("Invalid Redis URL")?;
    let manager = ConnectionManager::new(client)
        .await
        .context("Failed to connect to Redis")?;
    Ok(manager)
}