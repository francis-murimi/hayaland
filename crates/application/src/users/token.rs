use crate::errors::ApplicationError;
use async_trait::async_trait;
use uuid::Uuid;

/// Outbound port for generating authentication tokens.
#[async_trait]
pub trait TokenGenerator: Send + Sync {
    async fn generate(&self, user_id: Uuid) -> Result<String, ApplicationError>;
}
