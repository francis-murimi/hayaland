use crate::entities::EmailVerification;
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait EmailVerificationRepository: Send + Sync {
    async fn save(&self, verification: &EmailVerification) -> Result<(), DomainError>;
    async fn find_by_token(&self, token: &str) -> Result<Option<EmailVerification>, DomainError>;
    async fn mark_used(&self, token: &str) -> Result<(), DomainError>;
    async fn invalidate_unused_for_user(&self, user_id: Uuid) -> Result<(), DomainError>;
}
