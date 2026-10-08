use crate::{action::Action, resource::Resource};

pub struct Permission {
    pub resource: Resource,
    pub action: Action,
}
