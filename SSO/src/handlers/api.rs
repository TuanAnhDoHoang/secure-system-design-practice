use axum::{
    Form, Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use std::sync::Arc;

use crate::{
    db,
    error::AppError,
    models::{
        AuthorizeParams, RegisterRequest, RegisterResponse, TokenRequest, TokenResponse,
        VerifyQuery, VerifyResponse,
    },
    redis::{AuthCodeData, consume_auth_code, create_auth_code},
    services::AuthService,
    state::AppState,
    utils::{self, CodeChallengeMethod, validate_code_challenge, validate_code_verifier, verify_pkce},
};

/// POST /api/register - API Endpoint đăng ký tài khoản (JSON payload)
pub async fn register_api(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let email = payload.email.trim();
    let password = payload.password.trim();

    if email.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest(
            "Email and password are required".to_string(),
        ));
    }

    if !utils::email_validation(email) {
        return Err(AppError::BadRequest(
            "Email is invalid. Please use a valid email format".to_string(),
        ));
    }

    AuthService::register(&state, email, password).await?;

    Ok(Json(RegisterResponse {
        message: "Email has been sent to your email!".to_string(),
    }))
}

/// GET /verify_token - API Endpoint xác minh token qua Query param
pub async fn verify_token_api(
    State(state): State<Arc<AppState>>,
    Query(query): Query<VerifyQuery>,
) -> Result<Json<VerifyResponse>, AppError> {
    let email = query.email.trim();
    let token = query.token.trim();

    if email.is_empty() || token.is_empty() {
        return Err(AppError::BadRequest(
            "Email and token are required".to_string(),
        ));
    }

    let user_id = AuthService::verify_token(&state, email, token)
        .await
        .map_err(|e| match e {
            AppError::BadRequest(ref r) if r == "expired" => {
                AppError::BadRequest("Verify token has expired".to_string())
            }
            AppError::BadRequest(_) => AppError::BadRequest("Wrong verify token".to_string()),
            other => other,
        })?;

    Ok(Json(VerifyResponse { user_id }))
}

/// GET /authorize - OAuth2 Authorization Code + PKCE endpoint (RFC 6749 + RFC 7636)
///
/// Flow:
/// 1. Validate response_type == "code"
/// 2. Lookup client_id, kiểm tra redirect_uri hợp lệ
/// 3. Nếu client require_pkce → bắt buộc code_challenge (S256)
/// 4. Lưu auth_code + code_challenge vào Redis (TTL 10 phút)
/// 5. Redirect về redirect_uri?code=...&state=...
///
/// TODO: Hiện tại bỏ qua xác thực user (cần middleware session).
///       Trong production, phải đảm bảo user đã đăng nhập trước bước này.
pub async fn authorize_api(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuthorizeParams>,
) -> Result<Response, AppError> {
    // 1. Validate response_type
    if params.response_type != "code" {
        return Err(AppError::BadRequest(
            "unsupported_response_type: chỉ hỗ trợ 'code'".to_string(),
        ));
    }

    // 2. Lookup client
    let client = db::find_oauth_client_by_client_id(&state.db, &params.client_id)
        .await
        .map_err(|e| {
            log::error!("DB error khi tìm client: {:?}", e);
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::BadRequest("invalid_client: client_id không tồn tại".to_string()))?;

    // 3. Validate redirect_uri
    let redirect_uri = params.redirect_uri.trim().to_string();
    if !client.redirect_uris.contains(&redirect_uri) {
        return Err(AppError::BadRequest(
            "invalid_request: redirect_uri không hợp lệ".to_string(),
        ));
    }

    // 4. Validate PKCE
    let (code_challenge, challenge_method_str) = if client.require_pkce {
        // Bắt buộc phải có code_challenge khi require_pkce = true
        let challenge = params.code_challenge.as_deref().ok_or_else(|| {
            AppError::BadRequest(
                "invalid_request: code_challenge là bắt buộc với client này".to_string(),
            )
        })?;

        // Chỉ chấp nhận S256 cho security (không cho plain khi require_pkce)
        if params.code_challenge_method != "S256" {
            return Err(AppError::BadRequest(
                "invalid_request: chỉ hỗ trợ code_challenge_method=S256".to_string(),
            ));
        }

        if !validate_code_challenge(challenge) {
            return Err(AppError::BadRequest(
                "invalid_request: code_challenge không đúng định dạng BASE64URL".to_string(),
            ));
        }

        (challenge.to_string(), "S256".to_string())
    } else {
        // Client không require_pkce → PKCE vẫn có thể dùng tùy chọn
        match &params.code_challenge {
            Some(challenge) => {
                let method = CodeChallengeMethod::parse(&params.code_challenge_method)
                    .ok_or_else(|| {
                        AppError::BadRequest(
                            "invalid_request: code_challenge_method không hợp lệ".to_string(),
                        )
                    })?;
                if !validate_code_challenge(challenge) {
                    return Err(AppError::BadRequest(
                        "invalid_request: code_challenge không đúng định dạng".to_string(),
                    ));
                }
                let method_str = match method {
                    CodeChallengeMethod::S256 => "S256",
                    CodeChallengeMethod::Plain => "plain",
                }
                .to_string();
                (challenge.clone(), method_str)
            }
            // Không có PKCE → lưu chuỗi rỗng (sẽ skip verify ở /token)
            None => (String::new(), String::new()),
        }
    };

    // 5. Tạo auth code và lưu vào Redis
    // TODO: thay "demo_user" bằng user_id từ session thực
    let user_id = "demo_user".to_string();

    let auth_code_data = AuthCodeData {
        client_id: client.client_id.clone(),
        redirect_uri: redirect_uri.clone(),
        user_id,
        code_challenge,
        code_challenge_method: challenge_method_str,
        scope: params.scope.clone(),
    };

    let mut redis = state.redis.clone();
    let code = create_auth_code(&mut redis, auth_code_data)
        .await
        .map_err(|e| {
            log::error!("Redis error khi tạo auth code: {:?}", e);
            AppError::Internal(e.to_string())
        })?;

    // 6. Redirect về client với code (và state nếu có)
    let mut callback_url = format!("{}?code={}", redirect_uri, code);
    if let Some(state_val) = &params.state {
        callback_url.push_str(&format!("&state={}", state_val));
    }

    log::info!(
        "PKCE authorize thành công: client_id={}, redirect={}",
        client.client_id,
        redirect_uri
    );

    Ok(Redirect::to(&callback_url).into_response())
}

/// POST /token - Token exchange endpoint (RFC 6749 §4.1.3 + RFC 7636 §4.5)
///
/// Flow:
/// 1. Validate grant_type == "authorization_code"
/// 2. Consume auth_code từ Redis (one-time use)
/// 3. Verify client_id + redirect_uri khớp với lúc /authorize
/// 4. Nếu có code_challenge → verify PKCE (SHA256(verifier) == challenge)
/// 5. Verify client_secret cho confidential clients
/// 6. Phát hành access_token (JWT)
pub async fn token_api(
    State(state): State<Arc<AppState>>,
    Form(req): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // 1. Validate grant_type
    if req.grant_type != "authorization_code" {
        return Err(AppError::BadRequest(
            "unsupported_grant_type".to_string(),
        ));
    }

    // 2. Consume auth code từ Redis (xóa ngay sau khi lấy — chống replay)
    let mut redis = state.redis.clone();
    let code_data = consume_auth_code(&mut redis, &req.code)
        .await
        .map_err(|e| {
            log::error!("Redis error khi lấy auth code: {:?}", e);
            AppError::Internal(e.to_string())
        })?
        .ok_or_else(|| AppError::BadRequest("invalid_grant: auth code không hợp lệ hoặc đã hết hạn".to_string()))?;

    // 3. Validate client_id và redirect_uri khớp với lúc /authorize
    if code_data.client_id != req.client_id {
        return Err(AppError::BadRequest(
            "invalid_client: client_id không khớp".to_string(),
        ));
    }
    if code_data.redirect_uri != req.redirect_uri.trim() {
        return Err(AppError::BadRequest(
            "invalid_grant: redirect_uri không khớp".to_string(),
        ));
    }

    // 4. Lookup client để verify secret và lấy TTL
    let client = db::find_oauth_client_by_client_id(&state.db, &req.client_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("invalid_client".to_string()))?;

    // 5. Verify client_secret cho confidential clients
    if client.client_secret.is_some() {
        let provided_secret = req.client_secret.as_deref().unwrap_or("");
        let stored_secret = client.client_secret.as_deref().unwrap_or("");
        if provided_secret != stored_secret {
            return Err(AppError::BadRequest(
                "invalid_client: client_secret không đúng".to_string(),
            ));
        }
    }

    // 6. Verify PKCE nếu có code_challenge lưu từ bước /authorize
    if !code_data.code_challenge.is_empty() {
        let verifier = req.code_verifier.as_deref().ok_or_else(|| {
            AppError::BadRequest(
                "invalid_grant: code_verifier là bắt buộc".to_string(),
            )
        })?;

        if !validate_code_verifier(verifier) {
            return Err(AppError::BadRequest(
                "invalid_grant: code_verifier không đúng định dạng (43-128 ký tự)".to_string(),
            ));
        }

        let method = CodeChallengeMethod::parse(&code_data.code_challenge_method)
            .unwrap_or(CodeChallengeMethod::S256);

        if !verify_pkce(verifier, &code_data.code_challenge, &method) {
            log::warn!(
                "PKCE verification thất bại: client_id={}",
                req.client_id
            );
            return Err(AppError::BadRequest(
                "invalid_grant: code_verifier không khớp với code_challenge".to_string(),
            ));
        }
    } else if client.require_pkce {
        // Client require_pkce nhưng không có challenge → từ chối
        return Err(AppError::BadRequest(
            "invalid_grant: client này bắt buộc PKCE".to_string(),
        ));
    }

    // 7. Phát hành access_token (JWT HS512)
    let ttl_hours = (client.access_token_ttl / 3600).max(1) as i64;
    let access_token = utils::create_jwt(&code_data.user_id, &state.config.auth.jwt_secret, ttl_hours)
        .map_err(|e| {
            log::error!("Lỗi tạo JWT: {:?}", e);
            AppError::Internal(e.to_string())
        })?;

    log::info!(
        "Token issued: client_id={}, user_id={}",
        req.client_id,
        code_data.user_id
    );

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: client.access_token_ttl,
        refresh_token: None, // TODO: implement refresh token
        scope: if code_data.scope.is_empty() {
            None
        } else {
            Some(code_data.scope)
        },
    }))
}

/// GET /userinfo - OIDC Userinfo endpoint
pub async fn userinfo_api() -> Result<(StatusCode, String), AppError> {
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        "Userinfo endpoint under construction".to_string(),
    ))
}

/// GET /.well-know/jwks.json - JWKS public keys
pub async fn well_know_api() -> Result<(StatusCode, String), AppError> {
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        "JWKS endpoint under construction".to_string(),
    ))
}

/// GET /revoke - Token revocation endpoint
pub async fn revoke_api() -> Result<(StatusCode, String), AppError> {
    Ok((
        StatusCode::NOT_IMPLEMENTED,
        "Revoke endpoint under construction".to_string(),
    ))
}

/// GET /logout - Single Logout
pub async fn logout_api() -> Result<(StatusCode, String), AppError> {
    revoke_api().await
}

