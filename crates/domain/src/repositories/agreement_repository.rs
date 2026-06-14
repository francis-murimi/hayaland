use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::{Agreement, Signature};
use crate::errors::DomainError;

/// Outbound port for persisting and retrieving agreements and signatures.
#[async_trait]
pub trait AgreementRepository: Send + Sync {
    /// Save a new agreement.
    async fn create(&self, agreement: &Agreement) -> Result<(), DomainError>;

    /// Find the current agreement for a deal.
    async fn find_by_deal_id(&self, deal_id: Uuid) -> Result<Option<Agreement>, DomainError>;

    /// Find an agreement by its own id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Agreement>, DomainError>;

    /// Update an agreement (status, text, admin fields).
    async fn update(&self, agreement: &Agreement) -> Result<(), DomainError>;

    /// Record a signature.
    async fn create_signature(&self, signature: &Signature) -> Result<(), DomainError>;

    /// List all signatures for an agreement.
    async fn find_signatures_by_agreement(
        &self,
        agreement_id: Uuid,
    ) -> Result<Vec<Signature>, DomainError>;

    /// Check whether a specific party has already signed the agreement.
    async fn has_party_signed(
        &self,
        agreement_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError>;

    /// Count signatures recorded for an agreement.
    async fn count_signatures(&self, agreement_id: Uuid) -> Result<i64, DomainError>;
}
