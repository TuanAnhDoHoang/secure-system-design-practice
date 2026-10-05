use anyhow::Context;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;

use crate::config::DatabaseConfig;

/// Kết nối tới SurrealDB thông qua cấu hình DatabaseConfig
pub async fn connect_db(config: &DatabaseConfig) -> anyhow::Result<Surreal<Client>> {
    let address = format!("{}:{}", config.host, config.port);
    let db = Surreal::new::<Ws>(&address)
        .await
        .context(format!("Failed to connect to SurrealDB at {}", address))?;

    db.signin(Root {
        username: config.username.clone(),
        password: config.password.clone(),
    })
    .await
    .context("Failed to sign in to SurrealDB as Root")?;

    db.use_ns(&config.namespace)
        .use_db(&config.db_name)
        .await
        .context("Failed to select SurrealDB namespace and database")?;

    init_schema(&db).await.context("Failed to initialize SurrealDB schema")?;

    Ok(db)
}

/// Khởi tạo schema idempotent cho database trước khi thực hiện các query khác
pub async fn init_schema(db: &Surreal<Client>) -> anyhow::Result<()> {
    // Khởi tạo bảng 'user' dạng SCHEMALESS nếu chưa tồn tại
    let schema_query = "
        DEFINE TABLE IF NOT EXISTS user SCHEMALESS;
        DEFINE TABLE IF NOT EXISTS oauth_client SCHEMALESS;
    ";
    db.query(schema_query).await?;
    Ok(())
}
