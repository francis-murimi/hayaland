use crate::entities::EncryptionKey;
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Outbound port for persisting and retrieving encryption keys.
#[async_trait]
pub trait EncryptionKeyRepository: Send + Sync {
    /// Save a new encryption key.
    async fn create(&self, key: &EncryptionKey) -> Result<(), DomainError>;

    /// Find the single active encryption key, if any.
    async fn find_active(&self) -> Result<Option<EncryptionKey>, DomainError>;

    /// Find a key by its unique name.
    async fn find_by_name(&self, name: &str) -> Result<Option<EncryptionKey>, DomainError>;

    /// Deactivate all keys. Typically followed by creating a new active key.
    async fn deactivate_all(&self) -> Result<u64, DomainError>;

    /// Activate a specific key by id.
    async fn activate(&self, id: Uuid) -> Result<(), DomainError>;
}
