use crate::errors::ApplicationError;
use crate::users::dto::{DeactivateUserCommand, UserDto};
use domain::repositories::UserRepository;
use std::sync::Arc;
use time::OffsetDateTime;

pub struct DeactivateUser {
    repo: Arc<dyn UserRepository>,
}

impl DeactivateUser {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, cmd: DeactivateUserCommand) -> Result<UserDto, ApplicationError> {
        let mut user = self
            .repo
            .find_by_id(cmd.id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        user.is_active = false;
        user.updated_at = OffsetDateTime::now_utc();

        self.repo.update(&user).await?;
        Ok(UserDto::from(user))
    }
}
