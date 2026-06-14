use crate::agreements::dto::{AdminUpdateAgreementCommand, AgreementResult};
use crate::errors::ApplicationError;
use domain::entities::AgreementStatus;
use domain::repositories::{AgreementRepository, DealRepository};
use std::sync::Arc;
use tracing::{info, instrument};

use super::generate_agreement::map_agreement_to_result;

/// Allow platform admins to update administrative fields of an agreement.
#[derive(Clone)]
pub struct AdminUpdateAgreement {
    deal_repo: Arc<dyn DealRepository>,
    agreement_repo: Arc<dyn AgreementRepository>,
}

impl AdminUpdateAgreement {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
    ) -> Self {
        Self {
            deal_repo,
            agreement_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: AdminUpdateAgreementCommand,
    ) -> Result<AgreementResult, ApplicationError> {
        let mut agreement = self
            .agreement_repo
            .find_by_deal_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !agreement.can_be_admin_updated() {
            return Err(ApplicationError::Validation(vec![format!(
                "agreement status {} cannot be edited by an admin",
                agreement.agreement_status.as_str()
            )]));
        }

        let before = serde_json::json!({
            "governing_law": agreement.governing_law,
            "dispute_resolution": agreement.dispute_resolution,
            "effective_date": agreement.effective_date,
            "termination_date": agreement.termination_date,
            "auto_renew": agreement.auto_renew,
            "status": agreement.agreement_status.as_str(),
        });

        if let Some(governing_law) = cmd.governing_law {
            agreement.governing_law = Some(governing_law);
        }
        if let Some(dispute_resolution) = cmd.dispute_resolution {
            agreement.dispute_resolution = Some(dispute_resolution);
        }
        if cmd.effective_date.is_some() {
            agreement.effective_date = cmd.effective_date;
        }
        if cmd.termination_date.is_some() {
            agreement.termination_date = cmd.termination_date;
        }
        if let Some(auto_renew) = cmd.auto_renew {
            agreement.auto_renew = auto_renew;
        }
        if let Some(status) = cmd.status {
            match status {
                AgreementStatus::PendingSignatures | AgreementStatus::Terminated => {
                    if status == AgreementStatus::Terminated {
                        agreement.mark_terminated();
                    } else {
                        agreement.agreement_status = status;
                    }
                }
                _ => {
                    return Err(ApplicationError::Validation(vec![format!(
                        "admins may only set agreement status to PENDING_SIGNATURES or TERMINATED, got {}",
                        status.as_str()
                    )]));
                }
            }
        }

        let after = serde_json::json!({
            "governing_law": agreement.governing_law,
            "dispute_resolution": agreement.dispute_resolution,
            "effective_date": agreement.effective_date,
            "termination_date": agreement.termination_date,
            "auto_renew": agreement.auto_renew,
            "status": agreement.agreement_status.as_str(),
        });

        self.agreement_repo.update(&agreement).await?;

        self.deal_repo
            .record_history(
                cmd.deal_id,
                "AGREEMENT_ADMIN_EDIT",
                None,
                Some(serde_json::json!({
                    "admin_user_id": cmd.admin_user_id,
                    "reason": cmd.reason,
                    "before": before,
                    "after": after,
                })),
            )
            .await?;

        info!(
            agreement_id = %agreement.id,
            deal_id = %cmd.deal_id,
            "admin updated agreement metadata"
        );

        let signatures = self
            .agreement_repo
            .find_signatures_by_agreement(agreement.id)
            .await?;
        Ok(map_agreement_to_result(agreement, signatures))
    }
}
