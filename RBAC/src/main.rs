use axum::middleware::from_fn;
use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    handlers::{
        auth::{login, register},
        organization::{add_member, create_organization, remove_member, transfer_ownership},
        root::{health, home},
    },
    state::AppState,
};

mod action;
mod handlers;
mod permission;
mod resource;
mod role;
mod client;
mod service;
mod state;
mod surreal;
// client module declared below in src/client



#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let db = surreal::connect()
        .await
        .expect("failed to connect to SurrealDB");
    tracing::info!("connected to SurrealDB");

    let state = AppState::new(db);

    let app = Router::new()
        .route("/", get(home))
        .route("/health", get(health))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .nest(
            "/org",
            Router::new()
                .route("/", post(create_organization))
                .route("/{id}/members", post(add_member))
                .route("/{id}/members/remove", post(remove_member))
                .route("/{id}/transfer", post(transfer_ownership))
                .route_layer(from_fn(crate::service::token::auth_middleware)),
        )
        .route("/auth/refresh", post(crate::handlers::auth::refresh))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
