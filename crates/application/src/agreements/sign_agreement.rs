use crate::agreements::dto::{AgreementResult, SignAgreementCommand};
use crate::errors::ApplicationError;
use domain::entities::{AgreementStatus, Signature};
use domain::repositories::{AgreementRepository, DealRepository, PartyRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument};
use uuid::Uuid;

use super::generate_agreement::map_agreement_to_result;

/// Record a party's digital attestation on the current agreement version.
#[derive(Clone)]
pub struct SignAgreement {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    agreement_repo: Arc<dyn AgreementRepository>,
}

impl SignAgreement {
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

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: SignAgreementCommand,
    ) -> Result<AgreementResult, ApplicationError> {
        if !cmd.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        if !self
            .deal_repo
            .is_party_participant(cmd.deal_id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let agreement = self
            .agreement_repo
            .find_by_deal_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !agreement.can_be_signed() {
            return Err(ApplicationError::Validation(vec![format!(
                "agreement status {} does not allow signing",
                agreement.agreement_status.as_str()
            )]));
        }

        if self
            .agreement_repo
            .has_party_signed(agreement.id, cmd.actor_party_id)
            .await?
        {
            return Err(ApplicationError::Validation(vec![
                "party has already signed the current agreement version".to_string(),
            ]));
        }

        let party = self
            .party_repo
            .find_by_id(cmd.actor_party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let role = aggregate
            .participations
            .iter()
            .find(|p| p.party_id == cmd.actor_party_id)
            .map(|p| p.role.as_str().to_string())
            .unwrap_or_else(|| "participant".to_string());

        let signed_at = OffsetDateTime::now_utc();
        let attestation = Signature::attestation_string(
            party.display_name.as_str(),
            &role,
            agreement.id,
            agreement.version,
            &aggregate.deal.deal_reference,
            signed_at,
        );
        let signature_data = Signature::compute_signature_data(
            &agreement.agreement_text,
            cmd.actor_party_id,
            cmd.actor_user_id,
            signed_at,
            agreement.version,
            &attestation,
        );

        let signature = Signature::new(
            Uuid::now_v7(),
            agreement.id,
            cmd.actor_party_id,
            cmd.actor_user_id,
            cmd.signature_type,
            signature_data,
            cmd.ip_address,
            agreement.version,
        );

        self.agreement_repo.create_signature(&signature).await?;

        let signature_count = self.agreement_repo.count_signatures(agreement.id).await?;
        let mut agreement = agreement;
        let signatures = self
            .agreement_repo
            .find_signatures_by_agreement(agreement.id)
            .await?;

        if signature_count >= 3 && agreement.agreement_status != AgreementStatus::Signed {
            agreement.mark_signed();
            self.agreement_repo.update(&agreement).await?;
        }

        self.deal_repo
            .record_history(
                cmd.deal_id,
                "AGREEMENT_SIGNED",
                Some(cmd.actor_party_id),
                Some(serde_json::json!({
                    "agreement_id": agreement.id,
                    "version": agreement.version,
                    "signature_id": signature.id,
                    "signature_type": signature.signature_type.as_str(),
                })),
            )
            .await?;

        info!(
            signature_id = %signature.id,
            agreement_id = %agreement.id,
            party_id = %cmd.actor_party_id,
            "recorded agreement signature"
        );

        Ok(map_agreement_to_result(agreement, signatures))
    }
}
