use std::env;

use axum::{Json, extract::State};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use surrealdb::types::ToSql;

use crate::{
    role::Role,
    service::auth::{NewUser, UserInfo},
};

//Register
#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
}
#[derive(Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
}

//Login
#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}
#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenClaims {
    sub: String,
    username: String,
    exp: usize,
    token_type: String,
}

// fn role_to_string(role: &Role) -> String {
//     match role {
//        role::Role::OrganizationOwner(org_id) => format!("OrganizationOwner:{}", org_id),
//        role::Role::User => "User".to_string(),
//     }
// }

fn build_access_token(
    user_id: &str,
    username: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: (Utc::now() + Duration::minutes(15)).timestamp() as usize,
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

fn build_refresh_token(
    user_id: &str,
    username: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = TokenClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
        token_type: "refresh".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

//Handlers
pub async fn register(
    State(state): State<crate::state::SharedState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (axum::http::StatusCode, String)> {
    let hash_password = hash(req.password, DEFAULT_COST).map_err(|e| {
        tracing::error!("Failed to hash password: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Got error while register".into(),
        )
    })?;
    let new_user = NewUser::new(req.username, hash_password, vec![Role::User]);
    let user_created: Option<UserInfo> =
        state
            .db
            .create("users")
            .content(new_user)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create user: {}", e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Got error while register".into(),
                )
            })?;

    match user_created {
        Some(user) => Ok(Json(RegisterResponse {
            user_id: user.id().to_sql(),
        })),
        None => {
            tracing::error!("Failed to create user: No user returned from database");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Got error while register".into(),
            ))
        }
    }
}

pub async fn login(
    State(state): State<crate::state::SharedState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, String)> {
    let mut response = state
        .db
        .query("SELECT * FROM users WHERE username = $username LIMIT 1")
        .bind(("username", req.username.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Failed to find user: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Login failed".into(),
            )
        })?;

    let user: Option<UserInfo> = response.take(0).map_err(|e| {
        tracing::error!("Failed to deserialize user: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Login failed".into(),
        )
    })?;

    let user = user.ok_or_else(|| {
        tracing::warn!("Login attempt for unknown username: {}", req.username);
        (
            axum::http::StatusCode::UNAUTHORIZED,
            "Invalid username or password".to_string(),
        )
    })?;

    let password_matches = verify(&req.password, user.hash_password()).map_err(|e| {
        tracing::error!("Failed to verify user password: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Login failed".into(),
        )
    })?;

    if !password_matches {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "Invalid username or password".into(),
        ));
    }

    let jwt_secret = env::var("JWT_SECRET").map_err(|e| {
        tracing::error!("JWT secret not found: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Login failed".into(),
        )
    })?;

    let user_id = user.id().to_sql();
    let username = user.username().to_owned();
    let access_token = build_access_token(&user_id, &username, &jwt_secret).map_err(|e| {
        tracing::error!("Failed to create access token: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Login failed".into(),
        )
    })?;
    let refresh_token = build_refresh_token(&user_id, &username, &jwt_secret).map_err(|e| {
        tracing::error!("Failed to create refresh token: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Login failed".into(),
        )
    })?;

    Ok(Json(LoginResponse {
        user_id,
        username,
        access_token,
        refresh_token,
    }))
}

// Endpoint to refresh access token using a refresh token
#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
}

pub async fn refresh(
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, (axum::http::StatusCode, String)> {
    let jwt_secret = env::var("JWT_SECRET").map_err(|e| {
        tracing::error!("JWT secret not found: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Refresh failed".into(),
        )
    })?;

    let validation = jsonwebtoken::Validation::default();
    let token_data = jsonwebtoken::decode::<TokenClaims>(
        &req.refresh_token,
        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        tracing::debug!("refresh token decode error: {:?}", e);
        (
            axum::http::StatusCode::UNAUTHORIZED,
            "refresh_expired_or_invalid".into(),
        )
    })?;

    if token_data.claims.token_type != "refresh" {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "refresh_expired_or_invalid".into(),
        ));
    }

    // create new access token
    let new_claims = TokenClaims {
        sub: token_data.claims.sub.clone(),
        username: token_data.claims.username.clone(),
        exp: (Utc::now() + Duration::minutes(15)).timestamp() as usize,
        token_type: "access".to_string(),
    };

    let new_token = jsonwebtoken::encode(
        &Header::default(),
        &new_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("Failed to create access token: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Refresh failed".into(),
        )
    })?;

    Ok(Json(RefreshResponse {
        access_token: new_token,
    }))
}
