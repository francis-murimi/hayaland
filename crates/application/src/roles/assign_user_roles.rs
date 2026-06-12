use crate::errors::ApplicationError;
use crate::roles::dto::AssignUserRolesCommand;
use crate::users::dto::UserDto;
use domain::repositories::UserRepository;
use std::sync::Arc;
use tracing::{info, instrument, warn};

#[derive(Clone)]
pub struct AssignUserRoles {
    repo: Arc<dyn UserRepository>,
}

impl AssignUserRoles {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, cmd), fields(%cmd.user_id, roles = ?cmd.roles))]
    pub async fn execute(&self, cmd: AssignUserRolesCommand) -> Result<UserDto, ApplicationError> {
        let mut user = self
            .repo
            .find_by_id(cmd.user_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        if user.protected && !cmd.roles.iter().any(|r| r == "admin") {
            warn!(user_id = %user.id, "attempted to remove admin role from protected user");
            return Err(ApplicationError::CannotRemoveFirstAdmin);
        }

        user.roles = cmd.roles;
        user.updated_at = time::OffsetDateTime::now_utc();

        self.repo.update(&user).await?;
        info!(user_id = %user.id, roles = ?user.roles, "assigned user roles");
        Ok(UserDto::from(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_repo_with, test_user};
    use uuid::Uuid;

    #[tokio::test]
    async fn assigns_roles_to_user() {
        let user = test_user("user@example.com", "user", "password123");
        let id = user.id;
        let repo = test_repo_with(user);

        let result = AssignUserRoles::new(repo)
            .execute(AssignUserRolesCommand {
                user_id: id,
                roles: vec!["admin".to_string()],
            })
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().roles.contains(&"admin".to_string()));
    }

    #[tokio::test]
    async fn rejects_removing_admin_from_protected_user() {
        let mut user = test_user("first@example.com", "first", "password123");
        user.roles = vec!["admin".to_string()];
        user.protected = true;
        let id = user.id;
        let repo = test_repo_with(user);

        let result = AssignUserRoles::new(repo)
            .execute(AssignUserRolesCommand {
                user_id: id,
                roles: vec!["user".to_string()],
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::CannotRemoveFirstAdmin)
        ));
    }

    #[tokio::test]
    async fn returns_not_found_when_missing() {
        let repo = test_repo_with(test_user("other@example.com", "other", "password123"));
        let result = AssignUserRoles::new(repo)
            .execute(AssignUserRolesCommand {
                user_id: Uuid::now_v7(),
                roles: vec!["admin".to_string()],
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::NotFound)));
    }
}
