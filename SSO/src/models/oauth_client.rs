use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

/// Hỗ trợ các grant type OAuth2 / OIDC
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, SurrealValue)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    AuthorizationCode,
    ClientCredentials,
    RefreshToken,
}

impl Default for GrantType {
    fn default() -> Self {
        GrantType::AuthorizationCode
    }
}

impl std::fmt::Display for GrantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrantType::AuthorizationCode => write!(f, "authorization_code"),
            GrantType::ClientCredentials => write!(f, "client_credentials"),
            GrantType::RefreshToken => write!(f, "refresh_token"),
        }
    }
}

/// Thông tin OAuth2 / OIDC Client (Service Provider) đã đăng ký với IdP
#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct OAuthClient {
    pub id: RecordId,
    /// Tên hiển thị của ứng dụng client
    pub client_name: String,
    /// Client ID duy nhất (UUID)
    pub client_id: String,
    /// Client Secret (hash) — chỉ public clients để trống
    pub client_secret: Option<String>,
    /// Danh sách Redirect URIs hợp lệ (phải khớp khi gọi /authorize)
    pub redirect_uris: Vec<String>,
    /// Các scope được phép yêu cầu: openid, profile, email, ...
    pub allowed_scopes: Vec<String>,
    /// Grant types được phép: authorization_code, client_credentials, refresh_token
    pub grant_types: Vec<GrantType>,
    /// Bật PKCE bắt buộc (nên true cho public clients)
    pub require_pkce: bool,
    /// Thời gian sống của access token (giây)
    pub access_token_ttl: u64,
    /// Cho phép offline_access / refresh token không?
    pub allow_offline_access: bool,
    /// Địa chỉ liên hệ kỹ thuật
    pub contact_email: Option<String>,
    /// URL trang chủ của ứng dụng
    pub client_uri: Option<String>,
    /// Logo URL
    pub logo_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dùng để tạo mới OAuthClient
#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct NewOAuthClient {
    pub client_name: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub grant_types: Vec<GrantType>,
    pub require_pkce: bool,
    pub access_token_ttl: u64,
    pub allow_offline_access: bool,
    pub contact_email: Option<String>,
    pub client_uri: Option<String>,
    pub logo_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NewOAuthClient {
    pub fn from_form(form: &crate::handlers::ui::RegisterClientForm) -> Self {
        let now = Utc::now();
        let redirect_uris: Vec<String> = form
            .redirect_uris
            .split('\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let allowed_scopes: Vec<String> = form
            .allowed_scopes
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut grant_types = vec![GrantType::AuthorizationCode];
        if form.allow_offline_access {
            grant_types.push(GrantType::RefreshToken);
        }

        NewOAuthClient {
            client_name: form.client_name.trim().to_string(),
            client_id: Uuid::new_v4().to_string(),
            client_secret: if form.is_public_client {
                None
            } else {
                Some(Uuid::new_v4().to_string())
            },
            redirect_uris,
            allowed_scopes,
            grant_types,
            require_pkce: form.require_pkce,
            access_token_ttl: form.access_token_ttl.unwrap_or(3600),
            allow_offline_access: form.allow_offline_access,
            contact_email: if form.contact_email.trim().is_empty() {
                None
            } else {
                Some(form.contact_email.trim().to_string())
            },
            client_uri: if form.client_uri.trim().is_empty() {
                None
            } else {
                Some(form.client_uri.trim().to_string())
            },
            logo_uri: None,
            created_at: now,
            updated_at: now,
        }
    }
}
