use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisterResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VerifyQuery {
    pub email: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifyResponse {
    pub user_id: String,
}

/// Query params cho GET /authorize (RFC 6749 §4.1.1 + RFC 7636)
#[derive(Debug, Deserialize, Clone)]
pub struct AuthorizeParams {
    /// REQUIRED: "code" (authorization_code flow)
    pub response_type: String,
    /// REQUIRED: client_id đã đăng ký
    pub client_id: String,
    /// REQUIRED: redirect_uri phải khớp với danh sách đã đăng ký
    pub redirect_uri: String,
    /// RECOMMENDED: scope yêu cầu (vd: "openid profile email")
    #[serde(default)]
    pub scope: String,
    /// RECOMMENDED: state để chống CSRF trên phía client
    pub state: Option<String>,
    /// REQUIRED (với PKCE): code_challenge = BASE64URL(SHA256(verifier))
    pub code_challenge: Option<String>,
    /// REQUIRED (với PKCE): "S256" hoặc "plain" (mặc định "plain" theo RFC nhưng ta bắt buộc S256)
    #[serde(default = "default_challenge_method")]
    pub code_challenge_method: String,
}

fn default_challenge_method() -> String {
    "S256".to_string()
}

/// Response trả về khi /authorize thành công (redirect với code)
#[derive(Debug, Serialize)]
pub struct AuthorizeResponse {
    pub code: String,
    pub state: Option<String>,
}

/// Body cho POST /token (RFC 6749 §4.1.3 + RFC 7636 §4.5)
#[derive(Debug, Deserialize, Clone)]
pub struct TokenRequest {
    /// REQUIRED: "authorization_code"
    pub grant_type: String,
    /// REQUIRED: authorization code nhận từ /authorize
    pub code: String,
    /// REQUIRED: phải khớp với redirect_uri lúc /authorize
    pub redirect_uri: String,
    /// REQUIRED (confidential clients): client_id
    pub client_id: String,
    /// REQUIRED (confidential clients): client_secret
    pub client_secret: Option<String>,
    /// REQUIRED (PKCE): code_verifier gốc — server tự compute lại challenge để so sánh
    pub code_verifier: Option<String>,
}

/// Response từ POST /token
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claim {
    pub sub: String,
    pub exp: usize,
}

