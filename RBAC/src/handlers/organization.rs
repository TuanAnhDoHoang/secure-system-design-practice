use axum::http::StatusCode;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::{Deserialize, Serialize};
use surrealdb::types::ToSql;

use crate::action::Action;
use crate::resource::Resource;
use crate::service::auth::UserInfo;
use crate::service::organization::Organization;

const CREATE_MSG: &str = "Error during create organization";
const ADD_MEMBER_MSG: &str = "Error during add member";
const REMOVE_MEMBER_MSG: &str = "Error during remove member";
const TRANSFER_MSG: &str = "Error during transfer ownership";

#[derive(Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
}

#[derive(Deserialize)]
pub struct MemberRequest {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub new_owner: String,
}

#[derive(Serialize)]
pub struct OrgResponse {
    pub message: String,
}

pub async fn create_organization(
    State(state): State<crate::state::SharedState>,
    Extension(caller): Extension<String>,
    Json(req): Json<CreateOrgRequest>,
) -> Result<Json<OrgResponse>, (StatusCode, String)> {
    // Only allow caller to set themselves as owner
    if caller != req.owner {
        tracing::debug!(
            "create_organization unauthorized: caller {} != owner {}",
            caller,
            req.owner
        );
        return Err((StatusCode::FORBIDDEN, CREATE_MSG.into()));
    }
    // Check caller has permission to create organizations
    let mut response = state
        .db
        .query("SELECT * FROM users WHERE id = $id LIMIT 1")
        .bind(("id", caller.clone()))
        .await
        .map_err(|e| {
            tracing::debug!("create_organization user query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, CREATE_MSG.into())
        })?;

    let user: Option<UserInfo> = response.take(0).map_err(|e| {
        tracing::debug!("create_organization deserialize user error: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, CREATE_MSG.into())
    })?;

    let user = user.ok_or_else(|| (StatusCode::UNAUTHORIZED, CREATE_MSG.into()))?;

    // Check roles: require `Role::User` to be present and a Create permission on Resource::Organization
    let mut has_create_permission = false;
    let mut is_basic_user = false;
    for role in user.role() {
        match role {
            crate::role::Role::User => is_basic_user = true,
            _ => {}
        }
        for perm in role.get_permissions() {
            if std::mem::discriminant(&perm.action) == std::mem::discriminant(&Action::Create) {
                if let crate::resource::Resource::Organization = perm.resource {
                    has_create_permission = true;
                    break;
                }
            }
        }
        if has_create_permission {
            break;
        }
    }

    if !is_basic_user {
        tracing::debug!(
            "create_organization forbidden: caller {} missing User role",
            caller
        );
        return Err((StatusCode::FORBIDDEN, CREATE_MSG.into()));
    }

    if !has_create_permission {
        tracing::debug!(
            "create_organization forbidden: caller {} lacks create permission",
            caller
        );
        return Err((StatusCode::FORBIDDEN, CREATE_MSG.into()));
    }

    match Organization::new(req.name, req.description, &req.owner, &state).await {
        Ok(_org) => Ok(Json(OrgResponse {
            message: CREATE_MSG.into(),
        })),
        Err(e) => {
            tracing::debug!("create_organization error: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, CREATE_MSG.into()))
        }
    }
}

pub async fn add_member(
    State(state): State<crate::state::SharedState>,
    Path(org_id): Path<String>,
    Extension(caller): Extension<String>,
    Json(req): Json<MemberRequest>,
) -> Result<Json<OrgResponse>, (StatusCode, String)> {
    // load organization
    let mut response = state
        .db
        .query("SELECT * FROM organization WHERE id = $id LIMIT 1")
        .bind(("id", org_id.clone()))
        .await
        .map_err(|e| {
            tracing::debug!("add_member query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, ADD_MEMBER_MSG.into())
        })?;

    let org: Option<Organization> = response.take(0).map_err(|e| {
        tracing::debug!("add_member deserialize error: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, ADD_MEMBER_MSG.into())
    })?;

    let mut org = org.ok_or_else(|| (StatusCode::NOT_FOUND, ADD_MEMBER_MSG.into()))?;

    // authorization: only org owner can add members
    // authorization: only org owner or those with Invite permission on OwnOrganization(org_id)
    if caller != org.owner {
        // load caller user to check permissions
        let mut resp = state
            .db
            .query("SELECT * FROM users WHERE id = $id LIMIT 1")
            .bind(("id", caller.clone()))
            .await
            .map_err(|e| {
                tracing::debug!("add_member user query error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, ADD_MEMBER_MSG.into())
            })?;

        let caller_user: Option<UserInfo> = resp.take(0).map_err(|e| {
            tracing::debug!("add_member deserialize user error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, ADD_MEMBER_MSG.into())
        })?;

        let caller_user =
            caller_user.ok_or_else(|| (StatusCode::FORBIDDEN, ADD_MEMBER_MSG.into()))?;

        let mut has_invite = false;
        for role in caller_user.role() {
            for perm in role.get_permissions() {
                if std::mem::discriminant(&perm.action) == std::mem::discriminant(&Action::Invite) {
                    if let crate::resource::Resource::OwnOrganization(ref id) = perm.resource {
                        if id == &org.id.to_sql() {
                            has_invite = true;
                            break;
                        }
                    }
                    if let crate::resource::Resource::AllOrganizationMember = perm.resource {
                        has_invite = true;
                        break;
                    }
                }
            }
            if has_invite {
                break;
            }
        }

        if !has_invite {
            tracing::debug!(
                "add_member forbidden: caller {} lacks invite permission",
                caller
            );
            return Err((StatusCode::FORBIDDEN, ADD_MEMBER_MSG.into()));
        }
    }

    match org.add_member_persist(&state, req.user_id).await {
        Ok(_) => Ok(Json(OrgResponse {
            message: ADD_MEMBER_MSG.into(),
        })),
        Err(e) => {
            tracing::debug!("add_member persist error: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, ADD_MEMBER_MSG.into()))
        }
    }
}

pub async fn remove_member(
    State(state): State<crate::state::SharedState>,
    Path(org_id): Path<String>,
    Extension(caller): Extension<String>,
    Json(req): Json<MemberRequest>,
) -> Result<Json<OrgResponse>, (StatusCode, String)> {
    let mut response = state
        .db
        .query("SELECT * FROM organization WHERE id = $id LIMIT 1")
        .bind(("id", org_id.clone()))
        .await
        .map_err(|e| {
            tracing::debug!("remove_member query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, REMOVE_MEMBER_MSG.into())
        })?;

    let org: Option<Organization> = response.take(0).map_err(|e| {
        tracing::debug!("remove_member deserialize error: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, REMOVE_MEMBER_MSG.into())
    })?;

    let mut org = org.ok_or_else(|| (StatusCode::NOT_FOUND, REMOVE_MEMBER_MSG.into()))?;

    // authorization: only org owner can remove members
    if caller != org.owner {
        // check Kick permission on OwnOrganization(org_id) or AllOrganizationMember
        let mut resp = state
            .db
            .query("SELECT * FROM users WHERE id = $id LIMIT 1")
            .bind(("id", caller.clone()))
            .await
            .map_err(|e| {
                tracing::debug!("remove_member user query error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, REMOVE_MEMBER_MSG.into())
            })?;

        let caller_user: Option<UserInfo> = resp.take(0).map_err(|e| {
            tracing::debug!("remove_member deserialize user error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, REMOVE_MEMBER_MSG.into())
        })?;

        let caller_user =
            caller_user.ok_or_else(|| (StatusCode::FORBIDDEN, REMOVE_MEMBER_MSG.into()))?;

        let mut has_kick = false;
        for role in caller_user.role() {
            for perm in role.get_permissions() {
                if std::mem::discriminant(&perm.action) == std::mem::discriminant(&Action::Kick) {
                    if let crate::resource::Resource::OwnOrganization(ref id) = perm.resource {
                        if id == &org.id.to_sql() {
                            has_kick = true;
                            break;
                        }
                    }
                    if let crate::resource::Resource::AllOrganizationMember = perm.resource {
                        has_kick = true;
                        break;
                    }
                }
            }
            if has_kick {
                break;
            }
        }

        if !has_kick {
            tracing::debug!(
                "remove_member forbidden: caller {} lacks kick permission",
                caller
            );
            return Err((StatusCode::FORBIDDEN, REMOVE_MEMBER_MSG.into()));
        }
    }

    match org.remove_member_persist(&state, &req.user_id).await {
        Ok(_) => Ok(Json(OrgResponse {
            message: REMOVE_MEMBER_MSG.into(),
        })),
        Err(e) => {
            tracing::debug!("remove_member persist error: {:?}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, REMOVE_MEMBER_MSG.into()));
        }
    }
}

pub async fn transfer_ownership(
    State(state): State<crate::state::SharedState>,
    Path(org_id): Path<String>,
    Extension(caller): Extension<String>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<OrgResponse>, (StatusCode, String)> {
    // load org
    let mut response = state
        .db
        .query("SELECT * FROM organization WHERE id = $id LIMIT 1")
        .bind(("id", org_id.clone()))
        .await
        .map_err(|e| {
            tracing::debug!("transfer_ownership query error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, TRANSFER_MSG.into())
        })?;

    let org: Option<Organization> = response.take(0).map_err(|e| {
        tracing::debug!("transfer_ownership deserialize error: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, TRANSFER_MSG.into())
    })?;

    let mut org = org.ok_or_else(|| (StatusCode::NOT_FOUND, TRANSFER_MSG.into()))?;

    // authorization: only current owner can transfer ownership
    if caller != org.owner {
        // allow transfer if caller has Update permission for this OwnOrganization
        let mut resp = state
            .db
            .query("SELECT * FROM users WHERE id = $id LIMIT 1")
            .bind(("id", caller.clone()))
            .await
            .map_err(|e| {
                tracing::debug!("transfer_ownership user query error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, TRANSFER_MSG.into())
            })?;

        let caller_user: Option<UserInfo> = resp.take(0).map_err(|e| {
            tracing::debug!("transfer_ownership deserialize user error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, TRANSFER_MSG.into())
        })?;

        let caller_user =
            caller_user.ok_or_else(|| (StatusCode::FORBIDDEN, TRANSFER_MSG.into()))?;

        let mut has_update = false;
        for role in caller_user.role() {
            for perm in role.get_permissions() {
                if std::mem::discriminant(&perm.action) == std::mem::discriminant(&Action::Update) {
                    if let crate::resource::Resource::OwnOrganization(ref id) = perm.resource {
                        if id == &org.id.to_sql() {
                            has_update = true;
                            break;
                        }
                    }
                }
            }
            if has_update {
                break;
            }
        }

        if !has_update {
            tracing::debug!(
                "transfer_ownership forbidden: caller {} not owner and lacks update permission",
                caller
            );
            return Err((StatusCode::FORBIDDEN, TRANSFER_MSG.into()));
        }
    }

    if let Err(e) = org.transfer_ownership(req.new_owner.clone()) {
        tracing::debug!("transfer_ownership local error: {:?}", e);
        return Err((StatusCode::BAD_REQUEST, TRANSFER_MSG.into()));
    }

    // persist owner and members
    state
        .db
        .query("UPDATE organization SET owner = $owner, members = $members WHERE id = $id")
        .bind(("owner", req.new_owner))
        .bind(("members", org.members.clone()))
        .bind(("id", org.id.clone()))
        .await
        .map_err(|e| {
            tracing::debug!("transfer_ownership persist error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, TRANSFER_MSG.into())
        })?;

    Ok(Json(OrgResponse {
        message: TRANSFER_MSG.into(),
    }))
}
