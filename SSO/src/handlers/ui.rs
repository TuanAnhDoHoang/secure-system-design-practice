use askama::Template;
use axum::{
    extract::{Form, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::{self, OAuthClientRow},
    models::NewOAuthClient,
    services::AuthService,
    state::AppState,
    utils,
};

// ============================================================================
// 1. ASKAMA TEMPLATE STRUCTS
// ============================================================================

#[derive(Template, Default)]
#[template(path = "register.html")]
pub struct RegisterTemplate<'a> {
    pub csrf_token: &'a str,
    pub email: &'a str,
    pub email_error: Option<&'a str>,
    pub password_error: Option<&'a str>,
    pub confirm_password_error: Option<&'a str>,
    pub general_error: Option<&'a str>,
    pub success_message: Option<&'a str>,
}

#[derive(Template, Default)]
#[template(path = "verify.html")]
pub struct VerifyTokenTemplate<'a> {
    pub csrf_token: &'a str,
    pub email: &'a str,
    pub token: &'a str,
    pub error_type: Option<&'a str>,
    pub info_message: Option<&'a str>,
}

#[derive(Template, Default)]
#[template(path = "verify_result.html")]
pub struct VerifyResultTemplate<'a> {
    pub success: bool,
    pub reason: &'a str,
}

#[derive(Template, Default)]
#[template(path = "redirect.html")]
pub struct RedirectTemplate<'a> {
    pub client_name: &'a str,
    pub redirect_uri: &'a str,
    pub code: &'a str,
    pub state: Option<&'a str>,
}

#[derive(Template, Default)]
#[template(path = "login.html")]
pub struct LoginTemplate<'a> {
    pub csrf_token: &'a str,
    pub email: &'a str,
    pub error_message: Option<&'a str>,
    pub success_message: Option<&'a str>,
    pub return_to: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub csrf_token: String,
    pub clients: Vec<OAuthClientRow>,
    pub success_message: Option<String>,
    pub error_message: Option<String>,
    pub new_client_secret: Option<String>,
    pub pkce_count: usize,
}

// ============================================================================
// 2. FORM & QUERY DTOs
// ============================================================================

#[derive(Debug, Deserialize, Clone)]
pub struct LoginForm {
    pub csrf_token: String,
    pub email: String,
    pub password: String,
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegisterClientForm {
    pub csrf_token: String,
    pub client_name: String,
    pub redirect_uris: String,
    pub allowed_scopes: String,
    #[serde(default = "default_true")]
    pub require_pkce: bool,
    #[serde(default)]
    pub is_public_client: bool,
    #[serde(default)]
    pub allow_offline_access: bool,
    pub access_token_ttl: Option<u64>,
    #[serde(default)]
    pub contact_email: String,
    #[serde(default)]
    pub client_uri: String,
}

/// Helper cho serde default = true
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeleteClientForm {
    pub csrf_token: String,
    pub client_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct LoginQueryParams {
    pub email: Option<String>,
    pub info: Option<String>,
    pub error: Option<String>,
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegisterForm {
    pub csrf_token: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VerifyTokenForm {
    pub csrf_token: String,
    pub email: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResendCodeForm {
    pub csrf_token: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct VerifyQueryParams {
    pub email: Option<String>,
    pub token: Option<String>,
    pub error: Option<String>,
    pub info: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct VerifyResultQueryParams {
    pub status: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RedirectQueryParams {
    pub client_name: Option<String>,
    pub redirect_uri: Option<String>,
    pub code: Option<String>,
    pub state: Option<String>,
}

// ============================================================================
// 3. CSRF & CACHE-CONTROL HELPERS
// ============================================================================

pub const CSRF_COOKIE_NAME: &str = "idp_csrf_token";

pub fn extract_csrf_from_cookies(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(token) = cookie
            .strip_prefix(&format!("{}=", CSRF_COOKIE_NAME))
            .filter(|t| !t.is_empty())
        {
            return Some(token.to_string());
        }
    }
    None
}

pub const AUTH_COOKIE_NAME: &str = "auth_token";

pub fn extract_auth_from_cookies(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(token) = cookie
            .strip_prefix(&format!("{}=", AUTH_COOKIE_NAME))
            .filter(|t| !t.is_empty())
        {
            return Some(token.to_string());
        }
    }
    None
}

pub fn check_authenticated_user(headers: &HeaderMap, secret: &str) -> Option<crate::models::Claim> {
    let token = extract_auth_from_cookies(headers)?;
    utils::verify_jwt(&token, secret).ok()
}

pub fn create_csrf_cookie(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=3600",
        CSRF_COOKIE_NAME, token
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

pub fn get_or_generate_csrf(headers: &HeaderMap) -> (String, Option<HeaderValue>) {
    if let Some(token) = extract_csrf_from_cookies(headers) {
        (token, None)
    } else {
        let new_token = Uuid::new_v4().to_string();
        let cookie = create_csrf_cookie(&new_token);
        (new_token, Some(cookie))
    }
}

pub fn verify_csrf_token(
    headers: &HeaderMap,
    submitted_token: &str,
) -> Result<(), (StatusCode, String)> {
    let session_token = match extract_csrf_from_cookies(headers) {
        Some(tok) => tok,
        None => {
            log::warn!("CSRF Validation Failure: Missing session cookie");
            return Err((
                StatusCode::FORBIDDEN,
                "Lỗi bảo mật (403): Phiên làm việc không có CSRF token hợp lệ. Vui lòng tải lại trang."
                    .to_string(),
            ));
        }
    };

    if submitted_token.is_empty() || submitted_token != session_token {
        log::warn!("CSRF Validation Failure: Token mismatch");
        return Err((
            StatusCode::FORBIDDEN,
            "Lỗi bảo mật (403): CSRF token không khớp hoặc đã hết hạn. Vui lòng tải lại trang."
                .to_string(),
        ));
    }

    Ok(())
}

pub fn render_template_response<T: Template>(
    template: T,
    status: StatusCode,
    extra_cookie: Option<HeaderValue>,
) -> Response {
    match template.render() {
        Ok(html) => {
            let mut response = (status, Html(html)).into_response();
            let headers = response.headers_mut();
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
            );
            headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            if let Some(cookie) = extra_cookie {
                headers.append(header::SET_COOKIE, cookie);
            }
            response
        }
        Err(err) => {
            log::error!("Askama template render error: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
                "Lỗi hệ thống khi hiển thị giao diện.",
            )
                .into_response()
        }
    }
}

// ============================================================================
// 4. UI HANDLERS (CHỈ NHẬN REQUEST, GỌI SERVICE, TRẢ RESPONSE)
// ============================================================================

/// GET /login - Render giao diện trang đăng nhập
pub async fn login_ui_get(headers: HeaderMap, Query(query): Query<LoginQueryParams>) -> Response {
    let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
    let email = query.email.unwrap_or_default();
    let template = LoginTemplate {
        csrf_token: &csrf_token,
        email: &email,
        error_message: query.error.as_deref(),
        success_message: query.info.as_deref(),
        return_to: query.return_to.as_deref(),
    };
    render_template_response(template, StatusCode::OK, set_cookie)
}

/// GET /logout - Xóa cookie xác thực và đăng xuất người dùng
pub async fn logout_ui_get() -> Response {
    let clear_auth_cookie = HeaderValue::from_str(&format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        AUTH_COOKIE_NAME
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""));

    let mut response =
        Redirect::to("/login?info=Bạn+đã+đăng+xuất+thành+công.").into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_auth_cookie);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
    );
    response
}

/// POST /login - Xử lý đăng nhập người dùng
pub async fn login_ui_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(payload): Form<LoginForm>,
) -> Response {
    // 1. Kiểm tra CSRF token
    if let Err((status, msg)) = verify_csrf_token(&headers, &payload.csrf_token) {
        let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
        let template = LoginTemplate {
            csrf_token: &csrf_token,
            email: &payload.email,
            error_message: Some(&msg),
            return_to: payload.return_to.as_deref(),
            ..Default::default()
        };
        return render_template_response(template, status, set_cookie);
    }

    let email = payload.email.trim();

    // 2. Xác thực thông tin đăng nhập
    match AuthService::login(&state, email, &payload.password).await {
        Ok(user) => {
            // Tạo JWT token phiên đăng nhập (24 giờ)
            let jwt_token = match utils::create_jwt(&user.email, &state.config.auth.jwt_secret, 24)
            {
                Ok(token) => token,
                Err(e) => {
                    log::error!("Failed to create JWT token: {:?}", e);
                    let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
                    let template = LoginTemplate {
                        csrf_token: &csrf_token,
                        email,
                        error_message: Some("Lỗi hệ thống khi khởi tạo phiên làm việc."),
                        ..Default::default()
                    };
                    return render_template_response(
                        template,
                        StatusCode::INTERNAL_SERVER_ERROR,
                        set_cookie,
                    );
                }
            };

            // Thiết lập Cookie HttpOnly chứa auth_token
            let auth_cookie = HeaderValue::from_str(&format!(
                "auth_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
                jwt_token
            ))
            .unwrap_or_else(|_| HeaderValue::from_static(""));

            let redirect_target = payload
                .return_to
                .filter(|r| !r.is_empty() && r.starts_with('/'))
                .unwrap_or_else(|| "/dashboard".to_string());

            let mut response = Redirect::to(&redirect_target).into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, auth_cookie);
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
            );
            response
        }
        Err(e) => {
            let error_msg = match e {
                crate::error::AppError::BadRequest(msg) => msg,
                _ => "Đăng nhập thất bại. Vui lòng kiểm tra lại thông tin.".to_string(),
            };
            let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
            let template = LoginTemplate {
                csrf_token: &csrf_token,
                email,
                error_message: Some(&error_msg),
                return_to: payload.return_to.as_deref(),
                ..Default::default()
            };
            render_template_response(template, StatusCode::UNAUTHORIZED, set_cookie)
        }
    }
}

/// GET /register - Render form đăng ký
pub async fn register_ui_get(headers: HeaderMap) -> Response {
    let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
    let template = RegisterTemplate {
        csrf_token: &csrf_token,
        email: "",
        email_error: None,
        password_error: None,
        confirm_password_error: None,
        general_error: None,
        success_message: None,
    };
    render_template_response(template, StatusCode::OK, set_cookie)
}

/// POST /register - Xử lý đăng ký từ form giao diện người dùng
pub async fn register_ui_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(payload): Form<RegisterForm>,
) -> Response {
    // 1. Kiểm tra CSRF token
    if let Err((status, msg)) = verify_csrf_token(&headers, &payload.csrf_token) {
        let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
        let template = RegisterTemplate {
            csrf_token: &csrf_token,
            email: &payload.email,
            general_error: Some(&msg),
            ..Default::default()
        };
        return render_template_response(template, status, set_cookie);
    }

    // 2. Validate input fields
    if let Err(errors) =
        utils::validate_register_input(&payload.email, &payload.password, &payload.confirm_password)
    {
        let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
        let template = RegisterTemplate {
            csrf_token: &csrf_token,
            email: &payload.email,
            email_error: errors.email_error.as_deref(),
            password_error: errors.password_error.as_deref(),
            confirm_password_error: errors.confirm_password_error.as_deref(),
            ..Default::default()
        };
        return render_template_response(template, StatusCode::UNPROCESSABLE_ENTITY, set_cookie);
    }

    // 3. Gọi Service xử lý nghiệp vụ
    let email_trimmed = payload.email.trim();
    let result = AuthService::register(&state, email_trimmed, &payload.password).await;

    match result {
        Ok(_) => {
            let redirect_url = format!("/verify-email?email={}", email_trimmed);
            let mut response = Redirect::to(&redirect_url).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
            );
            response
        }
        Err(e) => {
            log::error!("Register service error: {:?}", e);
            let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
            let template = RegisterTemplate {
                csrf_token: &csrf_token,
                email: &payload.email,
                general_error: Some("Có lỗi xảy ra khi xử lý đăng ký. Vui lòng thử lại sau."),
                ..Default::default()
            };
            render_template_response(template, StatusCode::INTERNAL_SERVER_ERROR, set_cookie)
        }
    }
}

/// GET /verify-email hoặc /verify - Giao diện thông báo đã gửi email hoặc tự động kích hoạt tài khoản nếu có token
pub async fn verify_ui_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<VerifyQueryParams>,
) -> Response {
    let email = query.email.unwrap_or_default();
    let token = query.token.unwrap_or_default();
    let error_type = query.error;
    let info_message = query.info;

    let email_trimmed = email.trim();
    let token_trimmed = token.trim();

    // Nếu người dùng nhấn vào đường link từ email (có kèm token) -> Tự động xác thực tài khoản
    if !email_trimmed.is_empty() && !token_trimmed.is_empty() {
        return match AuthService::verify_token(&state, email_trimmed, token_trimmed).await {
            Ok(_) => {
                let mut response = Redirect::to("/verify-result?status=success").into_response();
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
                );
                response
            }
            Err(e) => {
                let reason = match e {
                    crate::error::AppError::BadRequest(ref r) if r == "expired" => "expired",
                    _ => "invalid",
                };
                let redirect_url = format!("/verify-result?status=error&reason={}", reason);
                let mut response = Redirect::to(&redirect_url).into_response();
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
                );
                response
            }
        };
    }

    let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
    let template = VerifyTokenTemplate {
        csrf_token: &csrf_token,
        email: email_trimmed,
        token: token_trimmed,
        error_type: error_type.as_deref(),
        info_message: info_message.as_deref(),
    };
    render_template_response(template, StatusCode::OK, set_cookie)
}

/// POST /verify - Xử lý submit token xác thực
pub async fn verify_ui_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(payload): Form<VerifyTokenForm>,
) -> Response {
    if let Err((status, msg)) = verify_csrf_token(&headers, &payload.csrf_token) {
        let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
        let template = VerifyTokenTemplate {
            csrf_token: &csrf_token,
            email: &payload.email,
            token: &payload.token,
            error_type: Some(&msg),
            ..Default::default()
        };
        return render_template_response(template, status, set_cookie);
    }

    let email = payload.email.trim();
    let token = payload.token.trim();

    if email.is_empty() || token.is_empty() {
        let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
        let template = VerifyTokenTemplate {
            csrf_token: &csrf_token,
            email,
            token,
            error_type: Some("invalid"),
            ..Default::default()
        };
        return render_template_response(template, StatusCode::BAD_REQUEST, set_cookie);
    }

    // Gọi Service xử lý
    match AuthService::verify_token(&state, email, token).await {
        Ok(_) => {
            let mut response = Redirect::to("/verify-result?status=success").into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
            );
            response
        }
        Err(e) => {
            let reason = match e {
                crate::error::AppError::BadRequest(ref r) if r == "expired" => "expired",
                _ => "invalid",
            };
            let redirect_url = format!("/verify-result?status=error&reason={}", reason);
            let mut response = Redirect::to(&redirect_url).into_response();
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
            );
            response
        }
    }
}

/// POST /resend-code - Gửi lại mã xác thực
pub async fn resend_code_ui_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(payload): Form<ResendCodeForm>,
) -> Response {
    if let Err((status, msg)) = verify_csrf_token(&headers, &payload.csrf_token) {
        let (csrf_token, set_cookie) = get_or_generate_csrf(&headers);
        let template = VerifyTokenTemplate {
            csrf_token: &csrf_token,
            email: &payload.email,
            error_type: Some(&msg),
            ..Default::default()
        };
        return render_template_response(template, status, set_cookie);
    }

    let email = payload.email.trim();
    if !email.is_empty() {
        let _ = AuthService::resend_verification(&state, email).await;
    }

    let redirect_url = format!(
        "/verify-email?email={}&info=Mã xác thực mới đã được gửi tới email của bạn",
        email
    );
    let mut response = Redirect::to(&redirect_url).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
    );
    response
}

/// GET /verify-result - Trang tĩnh kết quả xác thực
pub async fn verify_result_ui_get(Query(query): Query<VerifyResultQueryParams>) -> Response {
    let success = query.status.as_deref() == Some("success");
    let reason = query.reason.as_deref().unwrap_or("invalid");

    let template = VerifyResultTemplate { success, reason };
    render_template_response(template, StatusCode::OK, None)
}

/// GET /redirect - Trang chuyển tiếp về SP (auto form POST submit)
pub async fn redirect_ui_get(Query(query): Query<RedirectQueryParams>) -> Response {
    let client_name = query.client_name.as_deref().unwrap_or("Hệ thống ứng dụng");
    let redirect_uri = query.redirect_uri.as_deref().unwrap_or("/");
    let code = query.code.as_deref().unwrap_or("");
    let state = query.state.as_deref();

    let template = RedirectTemplate {
        client_name,
        redirect_uri,
        code,
        state,
    };
    render_template_response(template, StatusCode::OK, None)
}

// ============================================================================
// 5. DASHBOARD HANDLERS
// ============================================================================

/// GET /dashboard – Hiển thị dashboard quản lý OAuth Clients
pub async fn dashboard_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    // 1. Kiểm tra xác thực người dùng qua auth_token cookie
    if check_authenticated_user(&headers, &state.config.auth.jwt_secret).is_none() {
        let mut response =
            Redirect::to("/login?error=Vui+lòng+đăng+nhập+để+truy+cập+dashboard.").into_response();
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
        );
        return response;
    }

    let (csrf_token, csrf_cookie) = get_or_generate_csrf(&headers);

    let clients = match db::list_oauth_clients(&state.db).await {
        Ok(list) => list,
        Err(e) => {
            log::error!("Lỗi khi tải danh sách OAuth clients: {:?}", e);
            vec![]
        }
    };

    let success_message = params.get("success").cloned();
    let error_message = params.get("error").cloned();

    let pkce_count = clients.iter().filter(|c| c.require_pkce).count();
    let template = DashboardTemplate {
        csrf_token,
        clients,
        success_message,
        error_message,
        new_client_secret: None,
        pkce_count,
    };

    render_template_response(template, StatusCode::OK, csrf_cookie)
}

/// POST /dashboard/register-client – Đăng ký OAuth Client mới
pub async fn register_client_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<RegisterClientForm>,
) -> Response {
    if check_authenticated_user(&headers, &state.config.auth.jwt_secret).is_none() {
        return Redirect::to("/login?error=Vui+lòng+đăng+nhập+để+tiếp+tục.").into_response();
    }
    // Validate CSRF
    if let Err((status, msg)) = verify_csrf_token(&headers, &form.csrf_token) {
        let (csrf_token, csrf_cookie) = get_or_generate_csrf(&headers);
        let clients = db::list_oauth_clients(&state.db).await.unwrap_or_default();
        let pkce_count = clients.iter().filter(|c| c.require_pkce).count();
        let template = DashboardTemplate {
            csrf_token,
            clients,
            success_message: None,
            error_message: Some(msg),
            new_client_secret: None,
            pkce_count,
        };
        return render_template_response(template, status, csrf_cookie);
    }

    // Validate basic fields
    let client_name = form.client_name.trim().to_string();
    if client_name.is_empty() {
        return dashboard_error(&state, &headers, "Tên ứng dụng không được để trống.").await;
    }
    if form.redirect_uris.trim().is_empty() {
        return dashboard_error(&state, &headers, "Phải có ít nhất một Redirect URI.").await;
    }
    if form.allowed_scopes.trim().is_empty() {
        return dashboard_error(&state, &headers, "Phải chọn ít nhất một scope.").await;
    }

    // Build and create client
    let new_client = NewOAuthClient::from_form(&form);
    let client_secret = new_client.client_secret.clone();

    match db::create_oauth_client(&state.db, new_client).await {
        Ok(created) => {
            let (csrf_token, csrf_cookie) = get_or_generate_csrf(&headers);
            let clients = db::list_oauth_clients(&state.db).await.unwrap_or_default();
            let pkce_count = clients.iter().filter(|c| c.require_pkce).count();
            let template = DashboardTemplate {
                csrf_token,
                clients,
                success_message: Some(format!(
                    "Client '{}' (ID: {}) đã được tạo thành công!",
                    created.client_name, created.client_id
                )),
                error_message: None,
                new_client_secret: client_secret,
                pkce_count,
            };
            render_template_response(template, StatusCode::OK, csrf_cookie)
        }
        Err(e) => {
            log::error!("Lỗi khi tạo OAuth client: {:?}", e);
            dashboard_error(
                &state,
                &headers,
                "Lỗi hệ thống khi tạo client. Vui lòng thử lại.",
            )
            .await
        }
    }
}

/// POST /dashboard/delete – Xóa OAuth Client
pub async fn delete_client_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<DeleteClientForm>,
) -> Response {
    if check_authenticated_user(&headers, &state.config.auth.jwt_secret).is_none() {
        return Redirect::to("/login?error=Vui+lòng+đăng+nhập+để+tiếp+tục.").into_response();
    }
    if let Err((status, msg)) = verify_csrf_token(&headers, &form.csrf_token) {
        let (csrf_token, csrf_cookie) = get_or_generate_csrf(&headers);
        let clients = db::list_oauth_clients(&state.db).await.unwrap_or_default();
        let pkce_count = clients.iter().filter(|c| c.require_pkce).count();
        let template = DashboardTemplate {
            csrf_token,
            clients,
            success_message: None,
            error_message: Some(msg),
            new_client_secret: None,
            pkce_count,
        };
        return render_template_response(template, status, csrf_cookie);
    }

    match db::delete_oauth_client(&state.db, &form.client_id).await {
        Ok(true) => {
            Redirect::to("/dashboard?success=Client+đã+được+xóa+thành+công.").into_response()
        }
        Ok(false) => Redirect::to("/dashboard?error=Không+tìm+thấy+client+để+xóa.").into_response(),
        Err(e) => {
            log::error!("Lỗi khi xóa OAuth client: {:?}", e);
            Redirect::to("/dashboard?error=Lỗi+hệ+thống+khi+xóa+client.").into_response()
        }
    }
}

/// Helper: render dashboard với thông báo lỗi
async fn dashboard_error(state: &Arc<AppState>, headers: &HeaderMap, msg: &str) -> Response {
    let (csrf_token, csrf_cookie) = get_or_generate_csrf(headers);
    let clients = db::list_oauth_clients(&state.db).await.unwrap_or_default();
    let pkce_count = clients.iter().filter(|c| c.require_pkce).count();
    let template = DashboardTemplate {
        csrf_token,
        clients,
        success_message: None,
        error_message: Some(msg.to_string()),
        new_client_secret: None,
        pkce_count,
    };
    render_template_response(template, StatusCode::UNPROCESSABLE_ENTITY, csrf_cookie)
}

// ============================================================================
// 6. UNIT TESTS CHO VALIDATION, CSRF VÀ ASKAMA TEMPLATES
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::validate_register_input;

    #[test]
    fn test_valid_registration_input() {
        let result = validate_register_input("user@example.com", "StrongPass123", "StrongPass123");
        assert!(result.is_ok(), "Đầu vào hợp lệ phải trả về Ok");
    }

    #[test]
    fn test_empty_registration_input() {
        let result = validate_register_input("", "", "");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.email_error.is_some());
        assert!(errors.password_error.is_some());
        assert!(errors.confirm_password_error.is_some());
    }

    #[test]
    fn test_invalid_email_format() {
        let invalid_emails = vec![
            "plainaddress",
            "#@%^%#$@#$@#.com",
            "@example.com",
            "Joe Smith <email@example.com>",
            "email.example.com",
            "email@example@example.com",
        ];

        for email in invalid_emails {
            let result = validate_register_input(email, "StrongPass123", "StrongPass123");
            assert!(result.is_err(), "Email {} phải bị từ chối", email);
            let errors = result.unwrap_err();
            assert_eq!(
                errors.email_error.as_deref(),
                Some("Địa chỉ email không đúng định dạng.")
            );
        }
    }

    #[test]
    fn test_weak_passwords() {
        // Ít hơn 8 ký tự
        let res_short = validate_register_input("a@b.com", "Pass1", "Pass1");
        assert!(res_short.is_err());
        assert!(res_short.unwrap_err().password_error.is_some());

        // Thiếu chữ in hoa
        let res_no_upper = validate_register_input("a@b.com", "securepass123", "securepass123");
        assert!(res_no_upper.is_err());
        assert!(res_no_upper.unwrap_err().password_error.is_some());

        // Thiếu chữ in thường
        let res_no_lower = validate_register_input("a@b.com", "SECUREPASS123", "SECUREPASS123");
        assert!(res_no_lower.is_err());
        assert!(res_no_lower.unwrap_err().password_error.is_some());

        // Thiếu chữ số
        let res_no_digit = validate_register_input("a@b.com", "SecurePassword", "SecurePassword");
        assert!(res_no_digit.is_err());
        assert!(res_no_digit.unwrap_err().password_error.is_some());
    }

    #[test]
    fn test_password_mismatch() {
        let result =
            validate_register_input("user@example.com", "StrongPass123", "DifferentPass456");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.email_error.is_none());
        assert!(errors.password_error.is_none());
        assert_eq!(
            errors.confirm_password_error.as_deref(),
            Some("Mật khẩu xác nhận không khớp.")
        );
    }

    #[test]
    fn test_email_trimmed_properly() {
        let result = validate_register_input(
            "   user.test@example.com   ",
            "StrongPass123",
            "StrongPass123",
        );
        assert!(
            result.is_ok(),
            "Email có khoảng trắng thừa đầu đuôi phải được trim"
        );
    }

    #[test]
    fn test_csrf_cookie_generation_and_validation() {
        let token = "test-csrf-token-12345";
        let cookie_val = create_csrf_cookie(token);
        assert!(cookie_val.to_str().unwrap().contains(token));

        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!(
                "other=123; {}={}; more=456",
                CSRF_COOKIE_NAME, token
            ))
            .unwrap(),
        );

        // 1. Trích xuất thành công
        let extracted = extract_csrf_from_cookies(&headers);
        assert_eq!(extracted, Some(token.to_string()));

        // 2. Validate token khớp
        assert!(verify_csrf_token(&headers, token).is_ok());

        // 3. Validate token không khớp -> 403 Forbidden
        let mismatch_result = verify_csrf_token(&headers, "wrong-token");
        assert!(mismatch_result.is_err());
        let (status, _) = mismatch_result.unwrap_err();
        assert_eq!(status, StatusCode::FORBIDDEN);

        // 4. Validate khi không có cookie
        let empty_headers = HeaderMap::new();
        let no_cookie_res = verify_csrf_token(&empty_headers, token);
        assert!(no_cookie_res.is_err());
        assert_eq!(no_cookie_res.unwrap_err().0, StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_register_template_render() {
        let template = RegisterTemplate {
            csrf_token: "csrf-xyz-789",
            email: "invalid-user",
            email_error: Some("Địa chỉ email không đúng định dạng."),
            password_error: Some(
                "Mật khẩu phải có ít nhất 8 ký tự, bao gồm chữ hoa, chữ thường và chữ số.",
            ),
            confirm_password_error: Some("Mật khẩu xác nhận không khớp."),
            general_error: None,
            success_message: None,
        };
        let html = template
            .render()
            .expect("Template register.html phải render thành công");
        assert!(html.contains("name=\"csrf_token\" value=\"csrf-xyz-789\""));
        assert!(html.contains("autocomplete=\"new-password\""));
        assert!(html.contains("Địa chỉ email không đúng định dạng."));
        assert!(html.contains("Mật khẩu phải có ít nhất 8 ký tự"));
        assert!(html.contains("Mật khẩu xác nhận không khớp."));
        assert!(html.contains("action=\"/register\""));

        let success_template = RegisterTemplate {
            csrf_token: "csrf-xyz-789",
            email: "",
            email_error: None,
            password_error: None,
            confirm_password_error: None,
            general_error: None,
            success_message: Some("Nếu email hợp lệ, bạn sẽ nhận được thư xác nhận"),
        };
        let success_html = success_template
            .render()
            .expect("Render success register template");
        assert!(success_html.contains("Nếu email hợp lệ, bạn sẽ nhận được thư xác nhận"));
        assert!(!success_html.contains("action=\"/register\""));
    }

    #[test]
    fn test_verify_token_template_render() {
        let template = VerifyTokenTemplate {
            csrf_token: "csrf-abc-123",
            email: "user@example.com",
            token: "123456",
            error_type: Some("expired"),
            info_message: None,
        };
        let html = template
            .render()
            .expect("Template verify.html phải render thành công");
        assert!(html.contains("name=\"csrf_token\" value=\"csrf-abc-123\""));
        assert!(html.contains("user@example.com"));
        assert!(html.contains("Mã đã hết hạn"));
        assert!(html.contains("Gửi lại email xác thực"));
        assert!(html.contains("countdown"));

        let invalid_template = VerifyTokenTemplate {
            csrf_token: "csrf-abc-123",
            email: "user@example.com",
            token: "999999",
            error_type: Some("invalid"),
            info_message: None,
        };
        let invalid_html = invalid_template.render().unwrap();
        assert!(invalid_html.contains("Liên kết không hợp lệ"));
    }

    #[test]
    fn test_verify_result_template_render() {
        let success_tpl = VerifyResultTemplate {
            success: true,
            reason: "",
        };
        let success_html = success_tpl.render().expect("Render verify_result success");
        assert!(success_html.contains("Xác thực thành công!"));
        assert!(success_html.contains("Đăng nhập ngay"));
        assert!(success_html.contains("href=\"/login\""));

        let expired_tpl = VerifyResultTemplate {
            success: false,
            reason: "expired",
        };
        let expired_html = expired_tpl.render().expect("Render verify_result expired");
        assert!(expired_html.contains("Mã xác thực đã hết hạn"));
        assert!(expired_html.contains("Thử lại xác minh"));

        let used_tpl = VerifyResultTemplate {
            success: false,
            reason: "used",
        };
        let used_html = used_tpl.render().expect("Render verify_result used");
        assert!(used_html.contains("Mã đã được sử dụng"));
    }

    #[test]
    fn test_redirect_template_render() {
        let redirect_tpl = RedirectTemplate {
            client_name: "Cổng dịch vụ CRM",
            redirect_uri: "https://crm.example.com/oauth/callback",
            code: "auth_code_secret_abc123",
            state: Some("xyz_session_state"),
        };
        let html = redirect_tpl.render().expect("Render redirect template");
        assert!(html.contains("action=\"https://crm.example.com/oauth/callback\""));
        assert!(html.contains("method=\"POST\""));
        assert!(html.contains("name=\"code\" value=\"auth_code_secret_abc123\""));
        assert!(html.contains("name=\"state\" value=\"xyz_session_state\""));
        assert!(html.contains("document.getElementById('redirect-form')"));
        assert!(html.contains("form.submit()"));
        assert!(html.contains("<noscript>"));
        assert!(html.contains("Cổng dịch vụ CRM"));
    }

    #[test]
    fn test_login_template_render() {
        let template = LoginTemplate {
            csrf_token: "csrf-test-token",
            email: "test@example.com",
            error_message: Some("Email hoặc mật khẩu không chính xác"),
            success_message: None,
            return_to: None,
        };
        let html = template.render().expect("Render login template");
        assert!(html.contains("name=\"csrf_token\" value=\"csrf-test-token\""));
        assert!(html.contains("test@example.com"));
        assert!(html.contains("Email hoặc mật khẩu không chính xác"));
        assert!(html.contains("action=\"/login\""));
    }
}
