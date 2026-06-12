use crate::errors::ApplicationError;
use crate::users::dto::{DeactivateUserCommand, UserDto};
use domain::repositories::UserRepository;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument, warn};

#[derive(Clone)]
pub struct DeactivateUser {
    repo: Arc<dyn UserRepository>,
}

impl DeactivateUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, cmd), fields(%cmd.id))]
    pub async fn execute(&self, cmd: DeactivateUserCommand) -> Result<UserDto, ApplicationError> {
        let mut user = self
            .repo
            .find_by_id(cmd.id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        if user.has_role("admin") || user.protected {
            warn!(%cmd.id, "attempted to deactivate admin/protected user");
            return Err(ApplicationError::CannotDeactivateAdmin);
        }

        if !user.is_active {
            warn!(%cmd.id, "user already inactive");
        } else {
            user.is_active = false;
            user.updated_at = OffsetDateTime::now_utc();
        }

        self.repo.update(&user).await?;
        info!(id = %cmd.id, "deactivated user");
        Ok(UserDto::from(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_repo_with, test_user};
    use uuid::Uuid;

    #[tokio::test]
    async fn deactivates_user() {
        let user = test_user("active@example.com", "active", "password123");
        let id = user.id;
        let repo = test_repo_with(user);

        let result = DeactivateUser::new(repo)
            .execute(DeactivateUserCommand { id })
            .await;

        assert!(result.is_ok());
        assert!(!result.unwrap().is_active);
    }

    #[tokio::test]
    async fn rejects_deactivating_admin() {
        let mut user = test_user("admin@example.com", "admin", "password123");
        user.roles = vec!["admin".to_string()];
        let id = user.id;
        let repo = test_repo_with(user);

        let result = DeactivateUser::new(repo)
            .execute(DeactivateUserCommand { id })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::CannotDeactivateAdmin)
        ));
    }

    #[tokio::test]
    async fn returns_not_found_when_missing() {
        let repo = test_repo_with(test_user("other@example.com", "other", "password123"));
        let result = DeactivateUser::new(repo)
            .execute(DeactivateUserCommand { id: Uuid::now_v7() })
            .await;

        assert!(matches!(result, Err(ApplicationError::NotFound)));
    }
}
