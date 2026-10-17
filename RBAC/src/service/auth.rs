use getset::Getters;
use surrealdb::types::{RecordId, SurrealValue};

use crate::role::Role;

#[derive(SurrealValue)]
pub struct NewUser {
    username: String,
    hash_password: String,
    role: Vec<Role>,
}
impl NewUser {
    pub fn new(username: String, hash_password: String, role: Vec<Role>) -> Self {
        Self {
            username,
            hash_password,
            role,
        }
    }
}
#[derive(Getters, SurrealValue)]
#[getset(get = "pub")]
pub struct UserInfo {
    id: RecordId,
    username: String,
    hash_password: String,
    role: Vec<Role>,
}
