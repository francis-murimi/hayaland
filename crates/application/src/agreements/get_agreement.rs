use crate::agreements::dto::AgreementResult;
use crate::errors::ApplicationError;
use domain::repositories::{AgreementRepository, DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

use super::generate_agreement::map_agreement_to_result;

/// Retrieve an agreement by deal ID, enforcing visibility rules.
#[derive(Clone)]
pub struct GetAgreement {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    agreement_repo: Arc<dyn AgreementRepository>,
}

impl GetAgreement {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
    ) -> Self {
        Self {
            deal_repo,
            party_repo,
            agreement_repo,
        }
    }

    #[instrument(skip(self), fields(deal_id = %deal_id, user_id = %user_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<AgreementResult, ApplicationError> {
        let agreement = self
            .agreement_repo
            .find_by_deal_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !is_admin {
            let participations = self.deal_repo.find_participations_by_deal(deal_id).await?;
            let visible_party_ids: Vec<Uuid> = participations.iter().map(|p| p.party_id).collect();

            let is_member = match party_id {
                Some(pid) => visible_party_ids.contains(&pid),
                None => false,
            };

            if !is_member {
                let mut member_of_any = false;
                for pid in &visible_party_ids {
                    if self
                        .party_repo
                        .is_user_member_of_party(user_id, *pid)
                        .await?
                    {
                        member_of_any = true;
                        break;
                    }
                }
                if !member_of_any {
                    return Err(ApplicationError::DealNotFound);
                }
            }
        }

        let signatures = self
            .agreement_repo
            .find_signatures_by_agreement(agreement.id)
            .await?;

        info!(%deal_id, agreement_id = %agreement.id, "fetched agreement");
        Ok(map_agreement_to_result(agreement, signatures))
    }
}
