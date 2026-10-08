pub enum Resource {
    Organization,            //contain id of organization
    OwnOrganization(String), //contain id of organization
    Project(String),
    AllProjects,
    AllOrganizationMember,
    OrganizationMember(String), //contain id of organization
    Account,
    Task,
}
