pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod redis;
pub mod routes;
pub mod services;
pub mod state;
pub mod surreal;
pub mod utils;

pub use config::Config;
pub use error::AppError;
pub use routes::create_router;
pub use state::AppState;

use std::sync::Arc;

// Backward-compatibility bridges for existing handlers/tests
pub async fn handle_registration_logic(
    email: &str,
    password: &str,
    state: Arc<AppState>,
) -> anyhow::Result<()> {
    services::AuthService::register(&state, email, password)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_verify_logic(
    email: &str,
    token: &str,
    state: Arc<AppState>,
) -> Result<(), &'static str> {
    services::AuthService::verify_token(&state, email, token)
        .await
        .map(|_| ())
        .map_err(|e| match e {
            error::AppError::BadRequest(ref r) if r == "expired" => "expired",
            _ => "invalid",
        })
}

pub async fn handle_resend_logic(email: &str, state: Arc<AppState>) -> anyhow::Result<()> {
    services::AuthService::resend_verification(&state, email)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
