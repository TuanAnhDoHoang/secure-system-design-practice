use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::config::ClientConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub received_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingAuth {
    pub state: String,
    pub code_verifier: String,
    pub code_challenge: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub created_at: Instant,
}

#[derive(Debug, Default)]
pub struct OAuthStore {
    pub pending: HashMap<String, PendingAuth>,
    pub current_token: Option<OAuthToken>,
    pub current_user: Option<UserInfo>,
    pub logs: Vec<String>,
}

#[derive(Clone)]
pub struct OAuthManager {
    pub store: Arc<RwLock<OAuthStore>>,
    pub http_client: reqwest::Client,
}

impl OAuthManager {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(OAuthStore::default())),
            http_client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
        }
    }

    /// Sinh PKCE code_verifier (64 ký tự ngẫu nhiên, an toàn theo RFC 7636)
    pub fn generate_code_verifier() -> String {
        let raw = format!("{}{}{}", Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let hash = Sha256::digest(raw.as_bytes());
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Tính code_challenge theo chuẩn S256: BASE64URL(SHA256(verifier))
    pub fn generate_code_challenge(verifier: &str) -> String {
        let hash = Sha256::digest(verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Khởi tạo URL ủy quyền (Authorization URL)
    pub async fn create_authorize_url(&self, config: &ClientConfig) -> (String, String) {
        let state = Uuid::new_v4().to_string();
        let verifier = Self::generate_code_verifier();
        let challenge = Self::generate_code_challenge(&verifier);

        let pending = PendingAuth {
            state: state.clone(),
            code_verifier: verifier.clone(),
            code_challenge: challenge.clone(),
            redirect_uri: config.redirect_uri.clone(),
            client_id: config.client_id.clone(),
            created_at: Instant::now(),
        };

        {
            let mut store = self.store.write().await;
            store.pending.insert(state.clone(), pending);
            store.logs.push(format!(
                "[OAuth PKCE] Init flow: state={}, code_challenge={}",
                state, challenge
            ));
        }

        let url = format!(
            "{}/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            config.sso_url.trim_end_matches('/'),
            urlencoding::encode(&config.client_id),
            urlencoding::encode(&config.redirect_uri),
            urlencoding::encode(&config.scope),
            urlencoding::encode(&state),
            urlencoding::encode(&challenge)
        );

        (url, state)
    }

    /// Xử lý callback sau khi người dùng xác thực ở SSO và redirect về kèm ?code=&state=
    pub async fn handle_callback(
        &self,
        config: &ClientConfig,
        code: &str,
        state: &str,
    ) -> Result<OAuthToken, String> {
        let pending = {
            let mut store = self.store.write().await;
            store
                .pending
                .remove(state)
                .ok_or_else(|| format!("Invalid or expired state: {}", state))?
        };

        let token_url = format!("{}/token", config.sso_url.trim_end_matches('/'));

        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("client_id", &pending.client_id);
        params.insert("redirect_uri", &pending.redirect_uri);
        params.insert("code_verifier", &pending.code_verifier);

        let mut req = self.http_client.post(&token_url).form(&params);
        if let Some(ref secret) = config.client_secret {
            req = req.form(&[("client_secret", secret.as_str())]);
        }

        let res = req
            .send()
            .await
            .map_err(|e| format!("Token request failed: {}", e))?;

        let status = res.status();
        let body_text = res
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if !status.is_success() {
            let err_msg = format!("Token exchange error {}: {}", status, body_text);
            let mut store = self.store.write().await;
            store.logs.push(err_msg.clone());
            return Err(err_msg);
        }

        #[derive(Deserialize)]
        struct RawTokenResp {
            access_token: String,
            token_type: String,
            expires_in: u64,
            refresh_token: Option<String>,
            scope: Option<String>,
        }

        let raw: RawTokenResp = serde_json::from_str(&body_text)
            .map_err(|e| format!("Failed to parse token JSON: {} (raw: {})", e, body_text))?;

        let token = OAuthToken {
            access_token: raw.access_token,
            token_type: raw.token_type,
            expires_in: raw.expires_in,
            refresh_token: raw.refresh_token,
            scope: raw.scope,
            received_at: chrono::Utc::now().timestamp(),
        };

        {
            let mut store = self.store.write().await;
            store.current_token = Some(token.clone());
            store.logs.push(format!(
                "[OAuth PKCE] Token exchanged successfully: expires_in={}s",
                token.expires_in
            ));
        }

        // Tự động gọi /userinfo để lấy thông tin người dùng nếu có token
        let _ = self.fetch_userinfo(config, &token.access_token).await;

        Ok(token)
    }

    /// Gọi endpoint /userinfo với access_token
    pub async fn fetch_userinfo(
        &self,
        config: &ClientConfig,
        access_token: &str,
    ) -> Result<UserInfo, String> {
        let userinfo_url = format!("{}/userinfo", config.sso_url.trim_end_matches('/'));

        #[derive(Serialize)]
        struct UserInfoBody {
            access_token: String,
        }

        let res = self
            .http_client
            .get(&userinfo_url)
            .json(&UserInfoBody {
                access_token: access_token.to_string(),
            })
            .send()
            .await
            .map_err(|e| format!("Userinfo request failed: {}", e))?;

        let status = res.status();
        let body_text = res
            .text()
            .await
            .map_err(|e| format!("Failed to read userinfo body: {}", e))?;

        if !status.is_success() {
            let err_msg = format!("Userinfo endpoint error {}: {}", status, body_text);
            let mut store = self.store.write().await;
            store.logs.push(err_msg.clone());
            return Err(err_msg);
        }

        let user: UserInfo = serde_json::from_str(&body_text)
            .map_err(|e| format!("Failed to parse userinfo JSON: {}", e))?;

        {
            let mut store = self.store.write().await;
            store.current_user = Some(user.clone());
            store
                .logs
                .push(format!("[OAuth] Userinfo retrieved: email={}", user.email));
        }

        Ok(user)
    }

    pub async fn clear(&self) {
        let mut store = self.store.write().await;
        store.current_token = None;
        store.current_user = None;
        store.pending.clear();
        store.logs.push("[OAuth] Cleared session and tokens".to_string());
    }
}

// Module urlencoding mini helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
