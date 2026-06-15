use crate::errors::ApplicationError;
use crate::verifications::dto::{VerificationListResult, VerificationResult};
use domain::repositories::{PartyVerificationRepository, VerificationListFilters};
use std::sync::Arc;

#[derive(Clone)]
pub struct ListAdminVerifications {
    verification_repo: Arc<dyn PartyVerificationRepository>,
}

impl ListAdminVerifications {
    pub fn new(verification_repo: Arc<dyn PartyVerificationRepository>) -> Self {
        Self { verification_repo }
    }

    pub async fn execute(
        &self,
        query: crate::verifications::dto::AdminVerificationListQuery,
    ) -> Result<VerificationListResult, ApplicationError> {
        let filters = VerificationListFilters {
            status: query.status,
            verification_type: query.verification_type,
            party_id: query.party_id,
            limit: query.limit,
            offset: query.offset,
        };

        let domain_result = self.verification_repo.list(&filters).await?;

        Ok(VerificationListResult {
            verifications: domain_result
                .verifications
                .into_iter()
                .map(VerificationResult::from)
                .collect(),
            total: domain_result.total,
            limit: domain_result.limit,
            offset: domain_result.offset,
        })
    }
}
