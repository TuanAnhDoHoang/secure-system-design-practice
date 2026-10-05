use SSO::{
    config::Config, db::connect_db, redis::init_redis, routes::create_router,
    services::email_service::init_smtp, state::AppState,
};
use anyhow::Context;
use dotenv::dotenv;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Nạp biến môi trường từ .env
    dotenv().ok();
    env_logger::init();

    // 2. Nạp cấu hình ứng dụng
    let config = Config::from_env().context("Failed to load application configuration")?;

    // 3. Khởi tạo các kết nối hạ tầng (SurrealDB, Redis, Mailer)
    let db = connect_db(&config.database)
        .await
        .context("Failed to connect to SurrealDB database")?;
    let redis = init_redis(&config.redis)
        .await
        .context("Failed to connect to Redis cache")?;
    let mailer = init_smtp(&config.mail);

    // 4. Khởi tạo AppState dùng chung (Dependency Injection qua Arc)
    let state = Arc::new(AppState {
        db,
        redis,
        mailer,
        config: config.clone(),
    });

    // 5. Khởi tạo Router ứng dụng (UI Askama SSR + REST API)
    let app = create_router(state);

    // 6. Khởi động Axum HTTP Server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("Failed to bind TCP listener on {}", addr))?;

    log::info!("SSO Identity Provider running on http://{}", addr);
    axum::serve(listener, app)
        .await
        .context("Error occurred during HTTP server execution")?;

    Ok(())
}
