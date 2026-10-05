use axum::{
    Router,
    response::Redirect,
    routing::{get, post},
};
use std::sync::Arc;

use crate::{
    handlers::{api, ui},
    state::AppState,
};

/// Xây dựng toàn bộ Router cho ứng dụng SSO IdP
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Root → auto redirect to /register
        .route("/", get(|| async { Redirect::to("/register") }))
        // --- 1. Giao diện người dùng SSR (Askama Templates) ---
        // TRANG 1: Đăng ký (/register) - GET render form, POST xử lý submit
        .route(
            "/register",
            get(ui::register_ui_get).post(ui::register_ui_post),
        )
        // TRANG ĐĂNG NHẬP (/login)
        .route("/login", get(ui::login_ui_get).post(ui::login_ui_post))
        // TRANG DASHBOARD Quản lý OAuth Clients
        .route("/dashboard", get(ui::dashboard_get))
        .route("/dashboard/register-client", post(ui::register_client_post))
        .route("/dashboard/delete", post(ui::delete_client_post))
        // TRANG 2: Thông báo gửi email & link kích hoạt
        .route("/verify-email", get(ui::verify_ui_get))
        .route("/verify", get(ui::verify_ui_get).post(ui::verify_ui_post))
        .route("/resend-code", post(ui::resend_code_ui_post))
        // TRANG 3: Kết quả xác minh (Success / Failure) - Redirect 303 về đây
        .route("/verify-result", get(ui::verify_result_ui_get))
        // TRANG 4: Chuyển tiếp / Auto-submit form POST về SP (OAuth2 / OIDC callback)
        .route("/redirect", get(ui::redirect_ui_get))
        // --- 2. REST API Endpoints ---
        .route("/api/register", post(api::register_api))
        .route("/verify_token", get(api::verify_token_api))
        .route("/authorize", get(api::authorize_api))
        .route("/token", post(api::token_api))
        .route("/userinfo", get(api::userinfo_api))
        .route("/.well-know/jwks.json", get(api::well_know_api))
        .route("/revoke", get(api::revoke_api))
        .route("/logout", get(ui::logout_ui_get))
        .route("/api/logout", get(api::logout_api))
        // Gắn AppState dùng chung qua Arc
        .with_state(state)
}
