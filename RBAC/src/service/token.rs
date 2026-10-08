use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use serde::Deserialize;

#[derive(Deserialize, serde::Serialize, Clone)]
struct JwtClaims {
    sub: String,
    username: String,
    role: String,
    exp: usize,
    token_type: String,
}

pub async fn auth_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Check Authorization header
    let auth_header = match req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        Some(h) => h,
        None => return Err((StatusCode::UNAUTHORIZED, "Invalid access token".into())),
    };

    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, "Invalid access token".into()));
    }
    let token = &auth_header[7..];

    let jwt_secret = match std::env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Server error".into())),
    };

    let validation = jsonwebtoken::Validation::default();
    let token_data = match jsonwebtoken::decode::<JwtClaims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    ) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("auth middleware decode error: {:?}", e);
            // Return an explicit error when the access token is expired so clients know to refresh
            use jsonwebtoken::errors::ErrorKind;
            match e.kind() {
                ErrorKind::ExpiredSignature => {
                    return Err((StatusCode::UNAUTHORIZED, "token_expired".into()));
                }
                _ => return Err((StatusCode::UNAUTHORIZED, "Invalid access token".into())),
            }
        }
    };

    if token_data.claims.token_type != "access" {
        return Err((StatusCode::UNAUTHORIZED, "Invalid access token".into()));
    }

    // attach user id to request extensions for handlers
    req.extensions_mut().insert(token_data.claims.sub.clone());

    Ok(next.run(req).await)
}
