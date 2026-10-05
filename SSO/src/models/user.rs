use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, SurrealValue)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Inactive,
    Pending,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct Users {
    pub id: RecordId,
    pub email: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
