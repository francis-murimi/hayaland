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
