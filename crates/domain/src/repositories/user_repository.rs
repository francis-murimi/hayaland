use crate::entities::{Email, User, Username};
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Outbound port for persisting and retrieving users.
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError>;
    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError>;
    async fn list(
        &self,
        limit: i64,
        offset: i64,
        active_only: Option<bool>,
    ) -> Result<Vec<User>, DomainError>;
    async fn update(&self, user: &User) -> Result<(), DomainError>;
}
