use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{self, Response, StatusCode},
    middleware::Next,
};

use crate::{db, AppState};

pub async fn authorization_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, (StatusCode, String)> {
    let token = match req.headers().get(http::header::COOKIE) {
        Some(cookie_val) => {
            let cookie_header = cookie_val
                .to_str()
                .map_err(|_| (StatusCode::FORBIDDEN, "Invalid cookie header".to_string()))?;

            cookie_header
                .split(';')
                .map(str::trim)
                .find_map(|kv| {
                    let (key, value) = kv.split_once('=')?;
                    if key == "auth_token" {
                        Some(value.to_string())
                    } else {
                        None
                    }
                })
                .ok_or((StatusCode::FORBIDDEN, "Please sign in to continue".to_string()))?
        }
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                "Please add the JWT token to the header or cookie".to_string(),
            ));
        }
    };

    let claim = crate::utils::verify_jwt(&token, &state.config.auth.jwt_secret).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Unable to decode token".to_string(),
        )
    })?;

    let current_user = db::find_user_by_email(&state.db, &claim.sub)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error while checking authorization".to_string(),
            )
        })?
        .ok_or((StatusCode::UNAUTHORIZED, "You are not an authorized user".to_string()))?;

    req.extensions_mut().insert(current_user);
    Ok(next.run(req).await)
}