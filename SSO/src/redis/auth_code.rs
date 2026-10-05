use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Dữ liệu authorization code lưu trong Redis
/// TTL mặc định: 10 phút (RFC 6749 khuyến nghị tối đa)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCodeData {
    /// client_id đã khởi tạo authorization request
    pub client_id: String,
    /// redirect_uri đã được xác nhận
    pub redirect_uri: String,
    /// user_id đã xác thực thành công
    pub user_id: String,
    /// code_challenge từ /authorize request
    pub code_challenge: String,
    /// Phương thức: "S256" hoặc "plain"
    pub code_challenge_method: String,
    /// Scopes được cấp phép
    pub scope: String,
}

const AUTH_CODE_TTL_SECS: u64 = 600; // 10 phút

fn auth_code_key(code: &str) -> String {
    format!("auth_code:{}", code)
}

/// Tạo authorization code mới và lưu vào Redis với TTL 10 phút
pub async fn create_auth_code(
    redis: &mut ConnectionManager,
    data: AuthCodeData,
) -> anyhow::Result<String> {
    let code = Uuid::new_v4().to_string();
    let key = auth_code_key(&code);
    let value = serde_json::to_string(&data)?;
    redis
        .set_ex::<_, _, ()>(&key, value, AUTH_CODE_TTL_SECS)
        .await?;
    Ok(code)
}

/// Lấy dữ liệu authorization code từ Redis
pub async fn get_auth_code(
    redis: &mut ConnectionManager,
    code: &str,
) -> anyhow::Result<Option<AuthCodeData>> {
    let key = auth_code_key(code);
    let raw: Option<String> = redis.get(&key).await?;
    match raw {
        Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

/// Xóa authorization code sau khi đã dùng (one-time use — RFC 6749 §4.1.2)
pub async fn consume_auth_code(
    redis: &mut ConnectionManager,
    code: &str,
) -> anyhow::Result<Option<AuthCodeData>> {
    let data = get_auth_code(redis, code).await?;
    if data.is_some() {
        let key = auth_code_key(code);
        let _: () = redis.del(&key).await.unwrap_or(());
    }
    Ok(data)
}
