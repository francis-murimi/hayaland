use crate::errors::ApplicationError;
use crate::users::dto::{UpdateUserCommand, UserDto};
use domain::entities::{Email, Username};
use domain::repositories::UserRepository;
use std::sync::Arc;
use time::OffsetDateTime;

pub struct UpdateUser {
    repo: Arc<dyn UserRepository>,
}

impl UpdateUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, cmd: UpdateUserCommand) -> Result<UserDto, ApplicationError> {
        let mut user = self
            .repo
            .find_by_id(cmd.id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        if let Some(email) = cmd.email {
            let email = Email::new(&email).map_err(ApplicationError::from)?;
            if email != user.email {
                if let Some(existing) = self.repo.find_by_email(&email).await? {
                    if existing.id != user.id {
                        return Err(ApplicationError::DuplicateEmail);
                    }
                }
                user.email = email;
                user.updated_at = OffsetDateTime::now_utc();
            }
        }

        if let Some(username) = cmd.username {
            let username = Username::new(&username).map_err(ApplicationError::from)?;
            if username != user.username {
                if let Some(existing) = self.repo.find_by_username(&username).await? {
                    if existing.id != user.id {
                        return Err(ApplicationError::DuplicateUsername);
                    }
                }
                user.username = username;
                user.updated_at = OffsetDateTime::now_utc();
            }
        }

        self.repo.update(&user).await?;
        Ok(UserDto::from(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_repo_with, test_user, FakeRepo};
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn updates_email_and_username() {
        let user = test_user("old@example.com", "olduser", "password123");
        let id = user.id;
        let repo = test_repo_with(user);

        let result = UpdateUser::new(repo)
            .execute(UpdateUserCommand {
                id,
                email: Some("new@example.com".to_string()),
                username: Some("newuser".to_string()),
            })
            .await;

        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.email, "new@example.com");
        assert_eq!(dto.username, "newuser");
    }

    #[tokio::test]
    async fn rejects_duplicate_email() {
        let first = test_user("first@example.com", "first", "password123");
        let second = test_user("second@example.com", "second", "password123");
        let second_id = second.id;
        let repo = Arc::new(FakeRepo {
            users: Default::default(),
        });
        repo.create(&first).await.unwrap();
        repo.create(&second).await.unwrap();

        let result = UpdateUser::new(repo)
            .execute(UpdateUserCommand {
                id: second_id,
                email: Some("first@example.com".to_string()),
                username: None,
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::DuplicateEmail)));
    }

    #[tokio::test]
    async fn rejects_duplicate_username() {
        let first = test_user("first@example.com", "first", "password123");
        let second = test_user("second@example.com", "second", "password123");
        let second_id = second.id;
        let repo = Arc::new(FakeRepo {
            users: Default::default(),
        });
        repo.create(&first).await.unwrap();
        repo.create(&second).await.unwrap();

        let result = UpdateUser::new(repo)
            .execute(UpdateUserCommand {
                id: second_id,
                email: None,
                username: Some("first".to_string()),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::DuplicateUsername)));
    }

    #[tokio::test]
    async fn returns_not_found_when_missing() {
        let repo = test_repo_with(test_user("other@example.com", "other", "password123"));
        let result = UpdateUser::new(repo)
            .execute(UpdateUserCommand {
                id: Uuid::now_v7(),
                email: Some("new@example.com".to_string()),
                username: None,
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::NotFound)));
    }
}
