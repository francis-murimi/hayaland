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
