use axum::{
    Router, middleware,
    response::Redirect,
    routing::{get, post},
};
use std::sync::Arc;

use crate::{
    handlers::{api, ui},
    services::middleware::authorization_middleware,
    state::AppState,
};

/// Xây dựng toàn bộ Router cho ứng dụng SSO IdP
pub fn create_router(state: Arc<AppState>) -> Router {
    let auth_ui_routers = Router::new()
        .route(
            "/register",
            get(ui::register_ui_get).post(ui::register_ui_post),
        )
        .route("/login", get(ui::login_ui_get).post(ui::login_ui_post))
        .route("/verify-email", get(ui::verify_ui_get))
        .route("/verify", get(ui::verify_ui_get).post(ui::verify_ui_post))
        .route("/resend-code", post(ui::resend_code_ui_post))
        .route("/verify-result", get(ui::verify_result_ui_get))
        .route("/redirect", get(ui::redirect_ui_get));

    let ui_routers = Router::new()
        .route("/dashboard", get(ui::dashboard_get))
        .route("/dashboard/register-client", post(ui::register_client_post))
        .route("/dashboard/delete", post(ui::delete_client_post))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authorization_middleware,
        ));

    let api_routers = Router::new()
        .route("/api/register", post(api::register_api))
        .route("/verify_token", get(api::verify_token_api))
        .route("/authorize", get(api::authorize_api))
        .route("/token", post(api::token_api))
        .route("/userinfo", get(api::userinfo_api))
        .route("/.well-know/jwks.json", get(api::well_know_api))
        .route("/revoke", get(api::revoke_api))
        .route("/logout", get(ui::logout_ui_get))
        .route("/api/logout", get(api::logout_api));

    Router::new()
        // Root → auto redirect to /register
        .route("/", get(|| async { Redirect::to("/register") }))
        .nest("/auth", auth_ui_routers.clone())
        .merge(auth_ui_routers)
        .merge(api_routers)
        .merge(ui_routers)
        .with_state(state)
}
