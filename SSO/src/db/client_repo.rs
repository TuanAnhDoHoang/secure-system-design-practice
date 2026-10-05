use anyhow::anyhow;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use surrealdb::types::SurrealValue;

use crate::models::{NewOAuthClient, OAuthClient};

/// Struct nhẹ dùng khi listing clients (tránh deserialize toàn bộ)
#[derive(Debug, serde::Deserialize, SurrealValue)]
pub struct OAuthClientRow {
    pub id: surrealdb::types::RecordId,
    pub client_name: String,
    pub client_id: String,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub require_pkce: bool,
    pub allow_offline_access: bool,
    pub contact_email: Option<String>,
    pub client_uri: Option<String>,
}

/// Tạo mới OAuth Client trong database
pub async fn create_oauth_client(
    db: &Surreal<Client>,
    new_client: NewOAuthClient,
) -> anyhow::Result<OAuthClient> {
    let created: Option<OAuthClient> = db
        .create("oauth_client")
        .content::<NewOAuthClient>(new_client)
        .await?;

    created.ok_or_else(|| anyhow!("OAuth client was not created in database"))
}

/// Lấy danh sách tất cả OAuth Clients
pub async fn list_oauth_clients(db: &Surreal<Client>) -> anyhow::Result<Vec<OAuthClientRow>> {
    let query = "SELECT id, client_name, client_id, redirect_uris, allowed_scopes, require_pkce, allow_offline_access, contact_email, client_uri FROM oauth_client ORDER BY created_at DESC";
    let mut result = db.query(query).await?;
    let clients: Vec<OAuthClientRow> = result.take(0)?;
    Ok(clients)
}

/// Tìm OAuth Client theo client_id (UUID duy nhất)
pub async fn find_oauth_client_by_client_id(
    db: &Surreal<Client>,
    client_id: &str,
) -> anyhow::Result<Option<OAuthClient>> {
    let query = "SELECT * FROM oauth_client WHERE client_id = $client_id LIMIT 1";
    let mut result = db.query(query).bind(("client_id", client_id.trim())).await?;
    let clients: Vec<OAuthClient> = result.take(0)?;
    Ok(clients.into_iter().next())
}

/// Xóa OAuth Client theo record id string (dạng "oauth_client:xyz")
pub async fn delete_oauth_client(
    db: &Surreal<Client>,
    client_id: &str,
) -> anyhow::Result<bool> {
    // Tìm trước để lấy record id
    let client = find_oauth_client_by_client_id(db, client_id).await?;
    match client {
        Some(c) => {
            let _: Option<OAuthClient> = db.delete(c.id).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}
