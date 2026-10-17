use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub host: String,
    pub port: u16,
    pub sso_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scope: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        let host = env::var("CLIENT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("CLIENT_PORT")
            .or_else(|_| env::var("PORT"))
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4000);
        let sso_url = env::var("SSO_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let client_id = env::var("SSO_CLIENT_ID").unwrap_or_else(|_| "test-client".to_string());
        let client_secret = env::var("SSO_CLIENT_SECRET").ok();
        let redirect_uri = env::var("CLIENT_REDIRECT_URI")
            .unwrap_or_else(|_| format!("http://localhost:{}/callback", port));
        let scope = env::var("SSO_SCOPE").unwrap_or_else(|_| "openid email profile".to_string());

        Self {
            host,
            port,
            sso_url,
            client_id,
            client_secret,
            redirect_uri,
            scope,
        }
    }
}
