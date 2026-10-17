use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub mod auth_handler;
pub mod utils;

#[derive(Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub user_id: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize)]
struct RefreshRequest<'a> {
    refresh_token: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct RefreshResponse {
    pub access_token: String,
}

#[derive(Serialize)]
struct CreateOrgRequest<'a> {
    name: &'a str,
    description: Option<&'a str>,
    owner: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct OrgResponse {
    pub message: String,
}

pub async fn login(server: &str, username: &str, password: &str) -> anyhow::Result<LoginResponse> {
    let client = Client::new();
    let req = LoginRequest { username, password };
    let resp = client
        .post(format!("{}/auth/login", server))
        .json(&req)
        .send()
        .await?;

    let lr: LoginResponse = resp.json().await?;
    Ok(lr)
}

pub async fn refresh(server: &str, refresh_token: &str) -> anyhow::Result<RefreshResponse> {
    let client = Client::new();
    let req = RefreshRequest { refresh_token };
    let resp = client
        .post(format!("{}/auth/refresh", server))
        .json(&req)
        .send()
        .await?;

    let rr: RefreshResponse = resp.json().await?;
    Ok(rr)
}

pub async fn create_organization(
    server: &str,
    access_token: &str,
    name: &str,
    description: Option<&str>,
    owner: &str,
) -> anyhow::Result<OrgResponse> {
    let client = Client::new();
    let req = CreateOrgRequest { name, description, owner };
    let resp = client
        .post(format!("{}/org/", server))
        .bearer_auth(access_token)
        .json(&req)
        .send()
        .await?;

    let or: OrgResponse = resp.json().await?;
    Ok(or)
}

pub async fn add_member(server: &str, access_token: &str, org_id: &str, user_id: &str) -> anyhow::Result<OrgResponse> {
    let client = Client::new();
    let payload = json!({ "user_id": user_id });
    let resp = client
        .post(format!("{}/org/{}/members", server, org_id))
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await?;

    let or: OrgResponse = resp.json().await?;
    Ok(or)
}

pub async fn remove_member(server: &str, access_token: &str, org_id: &str, user_id: &str) -> anyhow::Result<OrgResponse> {
    let client = Client::new();
    let payload = json!({ "user_id": user_id });
    let resp = client
        .post(format!("{}/org/{}/members/remove", server, org_id))
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await?;

    let or: OrgResponse = resp.json().await?;
    Ok(or)
}

pub async fn transfer_ownership(server: &str, access_token: &str, org_id: &str, new_owner: &str) -> anyhow::Result<OrgResponse> {
    let client = Client::new();
    let payload = json!({ "new_owner": new_owner });
    let resp = client
        .post(format!("{}/org/{}/transfer", server, org_id))
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await?;

    let or: OrgResponse = resp.json().await?;
    Ok(or)
}

// keep a small helper for health check
pub async fn health(server: &str) -> anyhow::Result<String> {
    let resp = reqwest::get(format!("{}/health", server)).await?;
    let txt = resp.text().await?;
    Ok(txt)
}

#[derive(Serialize)]
struct RegisterRequest<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct RegisterResponse {
    pub user_id: String,
}

pub async fn register(server: &str, username: &str, password: &str) -> anyhow::Result<RegisterResponse> {
    let client = Client::new();
    let req = RegisterRequest { username, password };
    let resp = client
        .post(format!("{}/auth/register", server))
        .json(&req)
        .send()
        .await?;

    let rr: RegisterResponse = resp.json().await?;
    Ok(rr)
}