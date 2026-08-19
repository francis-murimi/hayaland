use crate::entities::PushToken;
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Outbound port for persisting and retrieving user push-notification tokens.
#[async_trait]
pub trait PushTokenRepository: Send + Sync {
    /// Register a new push token. If the same (user_id, device_token) pair already exists,
    /// update the metadata instead of creating a duplicate.
    async fn upsert(&self, token: &PushToken) -> Result<(), DomainError>;

    /// List all active tokens for a user.
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<PushToken>, DomainError>;

    /// Find a token by its id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PushToken>, DomainError>;

    /// Remove a single token by id.
    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Remove all tokens for a user.
    async fn delete_by_user(&self, user_id: Uuid) -> Result<u64, DomainError>;

    /// Update `last_used_at` to now.
    async fn touch(&self, id: Uuid) -> Result<(), DomainError>;
}
