use crate::entities::PasswordResetToken;
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait PasswordResetRepository: Send + Sync {
    async fn save(&self, token: &PasswordResetToken) -> Result<(), DomainError>;
    async fn find_by_token(&self, token: &str) -> Result<Option<PasswordResetToken>, DomainError>;
    async fn mark_used(&self, token: &str) -> Result<(), DomainError>;
    async fn invalidate_unused_for_user(&self, user_id: Uuid) -> Result<(), DomainError>;
}
