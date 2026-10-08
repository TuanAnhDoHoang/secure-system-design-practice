use surrealdb::types::SurrealValue;

use crate::action::Action;
use crate::permission::Permission;
use crate::resource::Resource;

#[derive(Debug, Clone, SurrealValue)]
pub struct CustomRole {
    id: u32,
    name: String,
}
#[derive(Debug, Clone, SurrealValue)]
pub enum Role {
    OrganizationOwner(String), //contain id of organization
    User,
}
impl Role {
    pub fn get_permissions(&self) -> Vec<Permission> {
        match self {
            Role::OrganizationOwner(org_id) => vec![
                //Organize realated permissions
                Permission {
                    resource: Resource::OwnOrganization(org_id.clone()), // Replace with actual organization ID
                    action: Action::Delete,
                },
                Permission {
                    resource: Resource::OwnOrganization(org_id.clone()), // Replace with actual organization ID
                    action: Action::Update,
                },
                Permission {
                    resource: Resource::OwnOrganization(org_id.clone()), // Replace with actual organization ID
                    action: Action::Read,
                },
                //Member related permissions
                Permission {
                    resource: Resource::AllOrganizationMember, // Replace with actual organization ID
                    action: Action::Invite,
                },
                Permission {
                    resource: Resource::AllOrganizationMember, // Replace with actual organization ID
                    action: Action::Kick,
                },
                //Project related permissions
                Permission {
                    resource: Resource::AllProjects, // Replace with actual organization ID
                    action: Action::Create,
                },
                Permission {
                    resource: Resource::AllProjects, // Replace with actual organization ID
                    action: Action::Update,
                },
                Permission {
                    resource: Resource::AllProjects, // Replace with actual organization ID
                    action: Action::Delete,
                },
            ],
            Role::User => vec![
                Permission {
                    resource: Resource::Account,
                    action: Action::Read,
                },
                Permission {
                    resource: Resource::Account,
                    action: Action::Update,
                },
                Permission {
                    resource: Resource::Account,
                    action: Action::Delete,
                },
                Permission {
                    resource: Resource::Organization,
                    action: Action::Create,
                },
            ],
        }
    }
}
