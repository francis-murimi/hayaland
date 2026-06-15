use crate::errors::ApplicationError;
use async_trait::async_trait;
use uuid::Uuid;

/// Outbound port used to request trust-score recalculation when a trust input changes.
#[async_trait]
pub trait TrustScoreRecalculationPort: Send + Sync {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError>;
}

/// No-op implementation used until the real trust-score use case is wired in.
pub struct NoOpTrustScoreRecalculation;

#[async_trait]
impl TrustScoreRecalculationPort for NoOpTrustScoreRecalculation {
    async fn request_recalculation(&self, _party_id: Uuid) -> Result<(), ApplicationError> {
        Ok(())
    }
}
