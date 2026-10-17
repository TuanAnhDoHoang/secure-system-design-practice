use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use tokio::sync::RwLock;

use super::{
    config::ClientConfig,
    oauth::OAuthManager,
    tester::RouteTester,
    ui::render_dashboard_html,
};

#[derive(Clone)]
pub struct ClientAppState {
    pub config: Arc<RwLock<ClientConfig>>,
    pub oauth: OAuthManager,
    pub tester: Arc<RouteTester>,
}

pub fn create_client_router(state: ClientAppState) -> Router {
    Router::new()
        // Giao diện người dùng
        .route("/", get(dashboard_handler))
        // OAuth 2.0 PKCE Callback Endpoint
        .route("/callback", get(oauth_callback_handler))
        // Cấu hình Client
        .route("/api/config", get(get_config_handler).post(update_config_handler))
        // Chạy kiểm thử Routes
        .route("/api/tests/run", post(run_tests_handler))
        // OAuth Actions
        .route("/api/oauth/start", get(oauth_start_handler))
        .route("/api/oauth/tokens", get(oauth_tokens_handler))
        .route("/api/oauth/clear", post(oauth_clear_handler))
        .route("/api/oauth/userinfo", get(oauth_userinfo_handler))
        // API Explorer Proxy
        .route("/api/explorer/send", post(explorer_send_handler))
        .with_state(state)
}

// 1. Dashboard UI
async fn dashboard_handler() -> Html<String> {
    Html(render_dashboard_html())
}

// 2. OAuth Callback
#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn oauth_callback_handler(
    State(state): State<ClientAppState>,
    Query(params): Query<CallbackParams>,
) -> Response {
    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        return Redirect::to(&format!(
            "/?tab=oauth&status=error&msg={}",
            urlencoding::encode(&format!("{}: {}", err, desc))
        ))
        .into_response();
    }

    let code = match params.code {
        Some(c) => c,
        None => {
            return Redirect::to("/?tab=oauth&status=error&msg=Missing%20authorization%20code")
                .into_response();
        }
    };

    let state_val = match params.state {
        Some(s) => s,
        None => {
            return Redirect::to("/?tab=oauth&status=error&msg=Missing%20state%20parameter")
                .into_response();
        }
    };

    let cfg = state.config.read().await.clone();
    match state.oauth.handle_callback(&cfg, &code, &state_val).await {
        Ok(_) => Redirect::to("/?tab=oauth&status=success").into_response(),
        Err(e) => Redirect::to(&format!(
            "/?tab=oauth&status=error&msg={}",
            urlencoding::encode(&e)
        ))
        .into_response(),
    }
}

// 3. Config Handlers
async fn get_config_handler(State(state): State<ClientAppState>) -> Json<ClientConfig> {
    let cfg = state.config.read().await.clone();
    Json(cfg)
}

#[derive(Deserialize)]
struct UpdateConfigPayload {
    sso_url: String,
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
    scope: String,
}

async fn update_config_handler(
    State(state): State<ClientAppState>,
    Json(payload): Json<UpdateConfigPayload>,
) -> StatusCode {
    let mut cfg = state.config.write().await;
    cfg.sso_url = payload.sso_url;
    cfg.client_id = payload.client_id;
    cfg.client_secret = payload.client_secret;
    cfg.redirect_uri = payload.redirect_uri;
    cfg.scope = payload.scope;
    StatusCode::OK
}

// 4. Test Runner Handler
#[derive(Deserialize)]
struct TestQuery {
    id: Option<String>,
}

async fn run_tests_handler(
    State(state): State<ClientAppState>,
    Query(query): Query<TestQuery>,
) -> Response {
    let cfg = state.config.read().await.clone();

    if let Some(test_id) = query.id {
        if let Some(res) = state.tester.run_single_test(&cfg, &test_id).await {
            return Json(res).into_response();
        } else {
            return (StatusCode::NOT_FOUND, "Test not found").into_response();
        }
    }

    let results = state.tester.run_all_tests(&cfg).await;
    Json(results).into_response()
}

// 5. OAuth Start
#[derive(Serialize)]
struct OAuthStartResponse {
    authorize_url: String,
    state: String,
}

async fn oauth_start_handler(State(state): State<ClientAppState>) -> Json<OAuthStartResponse> {
    let cfg = state.config.read().await.clone();
    let (authorize_url, state_val) = state.oauth.create_authorize_url(&cfg).await;
    Json(OAuthStartResponse {
        authorize_url,
        state: state_val,
    })
}

// 6. OAuth Tokens
#[derive(Serialize)]
struct OAuthTokensResponse {
    token: Option<super::oauth::OAuthToken>,
    user: Option<super::oauth::UserInfo>,
    logs: Vec<String>,
}

async fn oauth_tokens_handler(State(state): State<ClientAppState>) -> Json<OAuthTokensResponse> {
    let store = state.oauth.store.read().await;
    Json(OAuthTokensResponse {
        token: store.current_token.clone(),
        user: store.current_user.clone(),
        logs: store.logs.clone(),
    })
}

// 7. OAuth Clear
async fn oauth_clear_handler(State(state): State<ClientAppState>) -> StatusCode {
    state.oauth.clear().await;
    StatusCode::OK
}

// 8. OAuth Userinfo
async fn oauth_userinfo_handler(State(state): State<ClientAppState>) -> Response {
    let cfg = state.config.read().await.clone();
    let token_opt = {
        let store = state.oauth.store.read().await;
        store.current_token.as_ref().map(|t| t.access_token.clone())
    };

    let token = match token_opt {
        Some(t) => t,
        None => {
            return (StatusCode::BAD_REQUEST, "No access token in session").into_response();
        }
    };

    match state.oauth.fetch_userinfo(&cfg, &token).await {
        Ok(user) => Json(user).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

// 9. API Explorer Send Proxy
#[derive(Deserialize)]
struct ExplorerRequest {
    method: String,
    path: String,
    body: Option<String>,
}

#[derive(Serialize)]
struct ExplorerResponse {
    status: u16,
    status_text: String,
    duration_ms: u128,
    body: String,
}

async fn explorer_send_handler(
    State(state): State<ClientAppState>,
    Json(req): Json<ExplorerRequest>,
) -> Json<ExplorerResponse> {
    let cfg = state.config.read().await.clone();
    let clean_path = if req.path.starts_with('/') {
        req.path
    } else {
        format!("/{}", req.path)
    };
    let target_url = format!("{}{}", cfg.sso_url.trim_end_matches('/'), clean_path);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_default();

    let start = Instant::now();
    let method = match req.method.to_uppercase().as_str() {
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        _ => reqwest::Method::GET,
    };

    let mut builder = client.request(method, &target_url);
    if let Some(b) = req.body {
        if b.starts_with('{') {
            builder = builder
                .header("Content-Type", "application/json")
                .body(b);
        } else {
            builder = builder
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(b);
        }
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            let duration = start.elapsed().as_millis();
            let body_text = resp.text().await.unwrap_or_default();
            Json(ExplorerResponse {
                status: status.as_u16(),
                status_text: status.canonical_reason().unwrap_or("").to_string(),
                duration_ms: duration,
                body: body_text,
            })
        }
        Err(e) => Json(ExplorerResponse {
            status: 502,
            status_text: "Bad Gateway".to_string(),
            duration_ms: start.elapsed().as_millis(),
            body: format!("Error calling SSO: {}", e),
        }),
    }
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
