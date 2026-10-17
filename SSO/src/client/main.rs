use dotenv::dotenv;
use std::sync::Arc;
use tokio::sync::RwLock;

use SSO::client::{create_client_router, ClientAppState, ClientConfig, OAuthManager, RouteTester};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = ClientConfig::default();
    let addr = format!("{}:{}", config.host, config.port);

    log::info!("====================================================");
    log::info!("🚀 Starting SSO Test Client & OAuth 2.0 PKCE Runner");
    log::info!("   - Client URL:   http://{}", addr);
    log::info!("   - SSO Target:   {}", config.sso_url);
    log::info!("   - Client ID:    {}", config.client_id);
    log::info!("   - Redirect URI: {}", config.redirect_uri);
    log::info!("====================================================");

    let state = ClientAppState {
        config: Arc::new(RwLock::new(config)),
        oauth: OAuthManager::new(),
        tester: Arc::new(RouteTester::new()),
    };

    let app = create_client_router(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("----------------------------------------------------");
    println!("SSO Test Client is running at: http://{}", addr);
    println!("Open your browser and visit:  http://localhost:4000");
    println!("----------------------------------------------------");

    axum::serve(listener, app).await?;

    Ok(())
}
