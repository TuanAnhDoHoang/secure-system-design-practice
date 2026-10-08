use surrealdb::types::{RecordId, SurrealValue};

use crate::role::Role;

#[derive(Debug, Clone, SurrealValue)]
pub struct NewOrganization {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,         // single owner (admin)
    pub members: Vec<String>,  // vector of user ids
    pub projects: Vec<String>, // vector of project ids
}

#[derive(Debug, Clone, SurrealValue)]
pub struct Organization {
    pub id: RecordId,
    pub name: String,
    pub description: Option<String>,
    pub owner: String,         // single owner (admin)
    pub members: Vec<String>,  // vector of user ids
    pub projects: Vec<String>, // vector of project ids
}

// Người tạo tổ chức sẽ là chủ sở hữu (owner/admin)
// Admin có thể grant/revoke role cho các thành viên khác (role được lưu trong `User`)
// Admin có quyền truy cập đầy đủ tới projects, members và settings

impl NewOrganization {
    pub fn new(
        name: impl Into<String>,
        description: Option<String>,
        owner: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description,
            owner: owner.into(),
            members: Vec::new(),
            projects: Vec::new(),
        }
    }
}

impl Organization {
    pub async fn new(
        name: impl Into<String>,
        description: Option<String>,
        owner: &str,
        state: &crate::state::AppState,
    ) -> Result<Self, &'static str> {
        let new_organization = NewOrganization::new(name, description, owner);
        let organization_created: Option<Self> = state
            .db
            .create("organization")
            .content(new_organization)
            .await
            .map_err(|_| "Error during create organization")?;

        match organization_created {
            Some(org) => {
                // Assign the OrganizationOwner role to the owner (use service helper)
                let organization_owner_role = Role::OrganizationOwner(owner.into()); // Create a new OrganizationOwner role with the organization ID
                crate::service::role::assign_role_to_user(state, owner, &organization_owner_role)
                    .await
                    .map_err(|_| "Error assigning role")?;
                Ok(org)
            }
            None => Err("Failed to create organization"),
        }
    }

    pub fn add_member(&mut self, user_id: impl Into<String>) -> bool {
        let uid = user_id.into();
        if self.members.iter().any(|m| m == &uid) {
            return false;
        }
        self.members.push(uid);
        true
    }

    pub fn remove_member(&mut self, user_id: &str) -> bool {
        if let Some(pos) = self.members.iter().position(|m| m == user_id) {
            self.members.remove(pos);
            return true;
        }
        false
    }

    pub async fn add_member_persist(
        &mut self,
        state: &crate::state::AppState,
        user_id: impl Into<String>,
    ) -> Result<bool, &'static str> {
        let uid = user_id.into();
        if self.members.iter().any(|m| m == &uid) {
            return Ok(false);
        }

        // Build new members list and persist entire array (simpler and portable)
        let mut new_members = self.members.clone();
        new_members.push(uid.clone());

        state
            .db
            .query("UPDATE organization SET members = $members WHERE id = $id")
            .bind(("members", new_members.clone()))
            .bind(("id", self.id.clone()))
            .await
            .map_err(|_| "Failed to persist members")?;

        self.members = new_members;
        Ok(true)
    }

    pub async fn remove_member_persist(
        &mut self,
        state: &crate::state::AppState,
        user_id: &str,
    ) -> Result<bool, &'static str> {
        if !self.members.iter().any(|m| m == user_id) {
            return Ok(false);
        }

        let new_members: Vec<String> = self
            .members
            .iter()
            .cloned()
            .filter(|m| m != user_id)
            .collect();

        state
            .db
            .query("UPDATE organization SET members = $members WHERE id = $id")
            .bind(("members", new_members.clone()))
            .bind(("id", self.id.clone()))
            .await
            .map_err(|_| "Failed to persist members")?;

        self.members = new_members;
        Ok(true)
    }

    pub fn transfer_ownership(&mut self, new_owner: impl Into<String>) -> Result<(), &'static str> {
        let new_owner = new_owner.into();
        if self.owner == new_owner {
            return Err("already owner");
        }
        // ensure new owner is a member
        if !self.members.iter().any(|m| m == &new_owner) {
            self.members.push(new_owner.clone());
        }
        self.owner = new_owner;
        Ok(())
    }

    pub fn is_owner(&self, user_id: &str) -> bool {
        self.owner == user_id
    }

    pub fn has_member(&self, user_id: &str) -> bool {
        self.members.iter().any(|m| m == user_id)
    }
}
