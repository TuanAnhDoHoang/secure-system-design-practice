use crate::role::Role;

pub async fn assign_role_to_user(
    state: &crate::state::AppState,
    user_id: &str,
    role: &Role,
) -> Result<(), &'static str> {
    // Perform a single UPDATE query that appends the role into the user's role array
    // while avoiding a prior SELECT. We use SurrealQL expression to append and
    // deduplicate: `role = array::unique(role + [$role])`.
    state
        .db
        .query("UPDATE users SET role = array::unique((role ?? []) + [$role]) WHERE id = $id")
        .bind(("role", role.clone()))
        .bind(("id", user_id.to_string()))
        .await
        .map_err(|_| "Failed to update user roles")?;

    Ok(())
}
