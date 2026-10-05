use anyhow::anyhow;
use chrono::Utc;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

use surrealdb::types::SurrealValue;

use crate::models::{NewUser, UserStatus, Users};

#[derive(serde::Deserialize, SurrealValue)]
struct EmailCheck {
    #[allow(dead_code)]
    email: Option<String>,
}

/// Kiểm tra email đã tồn tại trong database hay chưa
/// Trả về: Ok(true) nếu email chưa tồn tại (có thể đăng ký), Ok(false) nếu email đã tồn tại
pub async fn is_email_available(db: &Surreal<Client>, email: &str) -> anyhow::Result<bool> {
    let query = "SELECT email FROM user WHERE email = $email LIMIT 1";
    let mut result = db.query(query).bind(("email", email)).await?;
    let records: Vec<EmailCheck> = result.take(0)?;
    Ok(records.is_empty())
}

/// Tìm người dùng theo email
pub async fn find_user_by_email(db: &Surreal<Client>, email: &str) -> anyhow::Result<Option<Users>> {
    let query = "SELECT * FROM user WHERE email = $email LIMIT 1";
    let mut result = db.query(query).bind(("email", email.trim())).await?;
    let users: Vec<Users> = result.take(0)?;
    Ok(users.into_iter().next())
}

/// Tạo bản ghi người dùng mới trong database
pub async fn create_user(
    db: &Surreal<Client>,
    email: &str,
    password_hash: &str,
) -> anyhow::Result<String> {
    let now = Utc::now();
    let new_user = NewUser {
        email: email.trim().to_string(),
        password_hash: password_hash.to_string(),
        status: UserStatus::Active,
        created_at: now,
        updated_at: now,
    };

    let created: Option<Users> = db.create("user").content::<NewUser>(new_user).await?;

    match created {
        Some(user) => match user.id.key {
            surrealdb::types::RecordIdKey::String(s) => Ok(s),
            surrealdb::types::RecordIdKey::Number(n) => Ok(n.to_string()),
            surrealdb::types::RecordIdKey::Uuid(u) => Ok(u.to_string()),
            _ => Ok(format!("{:?}", user.id.key)),
        },
        None => Err(anyhow!("User was not created in database")),
    }
}
