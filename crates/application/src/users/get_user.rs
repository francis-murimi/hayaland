use crate::errors::ApplicationError;
use crate::users::dto::UserDto;
use domain::repositories::UserRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct GetUser {
    repo: Arc<dyn UserRepository>,
}

impl GetUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: Uuid) -> Result<UserDto, ApplicationError> {
        let user = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        Ok(UserDto::from(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_repo_with, test_user};
    use uuid::Uuid;

    #[tokio::test]
    async fn returns_user_when_found() {
        let user = test_user("found@example.com", "founduser", "password123");
        let id = user.id;
        let repo = test_repo_with(user);

        let result = GetUser::new(repo).execute(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, id);
    }

    #[tokio::test]
    async fn returns_not_found_when_missing() {
        let repo = test_repo_with(test_user("other@example.com", "other", "password123"));
        let result = GetUser::new(repo).execute(Uuid::now_v7()).await;
        assert!(matches!(result, Err(ApplicationError::NotFound)));
    }
}
