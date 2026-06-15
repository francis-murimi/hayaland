use crate::agreements::{GenerateAgreement, GenerateAgreementCommand};
use crate::deals::create_deal::map_aggregate_to_result;
use crate::deals::dto::{DealResult, ExecuteTransitionCommand};
use crate::deals::validate_deal::{persist_validation, run_validation, status_is_good_or_better};
use crate::errors::ApplicationError;
use domain::entities::{AgreementStatus, DealStatus, ParticipationStatus, TermStatus};
use domain::repositories::{
    AgreementRepository, DealRepository, MilestoneRepository, PartyRepository, ReviewRepository,
};
use domain::services::ValidationConfig;
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Execute a state transition on a deal.
#[derive(Clone)]
pub struct ExecuteTransition {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    agreement_repo: Arc<dyn AgreementRepository>,
    milestone_repo: Option<Arc<dyn MilestoneRepository>>,
    review_repo: Option<Arc<dyn ReviewRepository>>,
    validation_config: ValidationConfig,
}

impl ExecuteTransition {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
        validation_config: ValidationConfig,
    ) -> Self {
        Self {
            deal_repo,
            party_repo,
            agreement_repo,
            milestone_repo: None,
            review_repo: None,
            validation_config,
        }
    }

    pub fn new_with_milestones(
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
        milestone_repo: Arc<dyn MilestoneRepository>,
        validation_config: ValidationConfig,
    ) -> Self {
        Self {
            deal_repo,
            party_repo,
            agreement_repo,
            milestone_repo: Some(milestone_repo),
            review_repo: None,
            validation_config,
        }
    }

    pub fn new_with_reviews(
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        agreement_repo: Arc<dyn AgreementRepository>,
        milestone_repo: Arc<dyn MilestoneRepository>,
        review_repo: Arc<dyn ReviewRepository>,
        validation_config: ValidationConfig,
    ) -> Self {
        Self {
            deal_repo,
            party_repo,
            agreement_repo,
            milestone_repo: Some(milestone_repo),
            review_repo: Some(review_repo),
            validation_config,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        cmd: ExecuteTransitionCommand,
    ) -> Result<DealResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let mut deal = aggregate.deal;
        let mut participations = aggregate.participations;

        // Verify actor is a member of a participating party (admins may act on behalf of a party).
        if !cmd.is_admin
            && !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
                .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let actor_participation = participations
            .iter_mut()
            .find(|p| p.party_id == cmd.actor_party_id)
            .ok_or(ApplicationError::Forbidden)?;

        // Transition-specific validations.
        match cmd.new_status {
            DealStatus::Negotiating => {
                if deal.deal_status != DealStatus::PendingReview {
                    return Err(ApplicationError::InvalidStateTransition {
                        from: deal.deal_status.as_str().to_string(),
                        to: cmd.new_status.as_str().to_string(),
                    });
                }
                actor_participation.participation_status = ParticipationStatus::Accepted;
                actor_participation.responded_at = Some(time::OffsetDateTime::now_utc());
                self.deal_repo
                    .update_participation(actor_participation)
                    .await?;
                // Move to negotiating only if all parties have accepted.
                let all_accepted = self
                    .deal_repo
                    .find_participations_by_deal(deal_id)
                    .await?
                    .into_iter()
                    .all(|p| {
                        if p.party_id == cmd.actor_party_id {
                            true // we just updated this one
                        } else {
                            p.participation_status == ParticipationStatus::Accepted
                        }
                    });
                if !all_accepted {
                    self.deal_repo
                        .record_history(
                            deal_id,
                            "PARTICIPATION_ACKNOWLEDGED",
                            Some(cmd.actor_party_id),
                            None,
                        )
                        .await?;
                    let aggregate = self.deal_repo.find_aggregate_by_id(deal_id).await?.unwrap();
                    return Ok(map_aggregate_to_result(aggregate));
                }
            }
            DealStatus::TermsLocked => {
                if deal.deal_status != DealStatus::Negotiating {
                    return Err(ApplicationError::InvalidStateTransition {
                        from: deal.deal_status.as_str().to_string(),
                        to: cmd.new_status.as_str().to_string(),
                    });
                }
                self.validate_terms_locked(deal_id).await?;
            }
            DealStatus::Committed => {
                if deal.deal_status != DealStatus::TermsLocked {
                    return Err(ApplicationError::InvalidStateTransition {
                        from: deal.deal_status.as_str().to_string(),
                        to: cmd.new_status.as_str().to_string(),
                    });
                }
                self.validate_commit(deal_id, cmd.acknowledge_warnings)
                    .await?;
                self.require_signed_agreement(deal_id).await?;
            }
            DealStatus::Executing => {
                if deal.deal_status != DealStatus::Committed {
                    return Err(ApplicationError::InvalidStateTransition {
                        from: deal.deal_status.as_str().to_string(),
                        to: cmd.new_status.as_str().to_string(),
                    });
                }
                self.ensure_milestones_present(deal_id).await?;
            }
            DealStatus::Completed => {
                if deal.deal_status != DealStatus::Executing {
                    return Err(ApplicationError::InvalidStateTransition {
                        from: deal.deal_status.as_str().to_string(),
                        to: cmd.new_status.as_str().to_string(),
                    });
                }
                self.ensure_all_milestones_verified(deal_id).await?;
                self.ensure_all_reviews_submitted(deal_id).await?;
            }
            DealStatus::Cancelled => {
                // Allow cancellation from most active states by any participant.
                if deal.deal_status.is_terminal() {
                    return Err(ApplicationError::InvalidStateTransition {
                        from: deal.deal_status.as_str().to_string(),
                        to: cmd.new_status.as_str().to_string(),
                    });
                }
            }
            _ => {
                // Other transitions require admin or specific preconditions.
                return Err(ApplicationError::Validation(vec![format!(
                    "transition to {} is not supported via this endpoint",
                    cmd.new_status.as_str()
                )]));
            }
        }

        deal.transition(cmd.new_status)
            .map_err(ApplicationError::from)?;

        if cmd.new_status == DealStatus::Executing {
            deal.actual_start_date = Some(time::OffsetDateTime::now_utc().date());
        }
        if cmd.new_status == DealStatus::Completed {
            deal.actual_end_date = Some(time::OffsetDateTime::now_utc().date());
        }

        for participation in &participations {
            self.deal_repo.update_participation(participation).await?;
        }
        self.deal_repo.update(&deal).await?;

        if cmd.new_status == DealStatus::TermsLocked {
            self.generate_agreement(deal_id, cmd.actor_user_id, cmd.actor_party_id)
                .await?;
        }

        self.deal_repo
            .record_history(
                deal_id,
                &format!("DEAL_TRANSITIONED_TO_{}", cmd.new_status.as_str()),
                Some(cmd.actor_party_id),
                cmd.reason.map(|r| serde_json::json!({ "reason": r })),
            )
            .await?;

        info!(%deal_id, new_status = %cmd.new_status.as_str(), "executed deal transition");
        Ok(map_aggregate_to_result(
            domain::repositories::DealAggregate {
                deal,
                participations,
            },
        ))
    }

    async fn ensure_milestones_present(&self, deal_id: Uuid) -> Result<(), ApplicationError> {
        let repo = self.milestone_repo.as_ref().ok_or_else(|| {
            ApplicationError::Validation(vec![
                "milestone repository is required to start execution".to_string(),
            ])
        })?;
        let count = repo.count_by_deal(deal_id).await?;
        if count == 0 {
            return Err(ApplicationError::Validation(vec![
                "at least one milestone is required before executing".to_string(),
            ]));
        }
        Ok(())
    }

    async fn ensure_all_milestones_verified(&self, deal_id: Uuid) -> Result<(), ApplicationError> {
        let repo = self.milestone_repo.as_ref().ok_or_else(|| {
            ApplicationError::Validation(vec![
                "milestone repository is required to complete the deal".to_string(),
            ])
        })?;
        let total = repo.count_by_deal(deal_id).await?;
        if total == 0 {
            return Err(ApplicationError::Validation(vec![
                "deal has no milestones to complete".to_string(),
            ]));
        }
        let verified = repo.count_verified_by_deal(deal_id).await?;
        if verified != total {
            return Err(ApplicationError::Validation(vec![
                "all milestones must be verified before completing the deal".to_string(),
            ]));
        }
        Ok(())
    }

    async fn ensure_all_reviews_submitted(&self, deal_id: Uuid) -> Result<(), ApplicationError> {
        let review_repo = self.review_repo.as_ref().ok_or_else(|| {
            ApplicationError::Infrastructure("review repository not configured".to_string())
        })?;

        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        let pairs: Vec<(Uuid, _)> = aggregate
            .participations
            .iter()
            .map(|p| (p.party_id, p.role))
            .collect();

        let missing = review_repo
            .find_missing_review_pairs(deal_id, &pairs)
            .await?;

        if !missing.is_empty() {
            return Err(ApplicationError::Validation(vec![
                "deal cannot be completed until all parties have reviewed each other".to_string(),
            ]));
        }

        Ok(())
    }

    async fn validate_terms_locked(&self, deal_id: Uuid) -> Result<(), ApplicationError> {
        self.deal_repo
            .find_value_distribution_by_deal(deal_id)
            .await?
            .ok_or_else(|| ApplicationError::WinWinWinValidationFailed {
                violations: vec!["value distribution is required before locking terms".to_string()],
            })?;

        let terms = self.deal_repo.find_terms_by_deal(deal_id).await?;
        let mandatory_unaccepted = terms
            .iter()
            .any(|t| t.is_mandatory && t.negotiation_status != TermStatus::Accepted);
        if mandatory_unaccepted {
            return Err(ApplicationError::WinWinWinValidationFailed {
                violations: vec!["all mandatory terms must be accepted".to_string()],
            });
        }

        let result = run_validation(&*self.deal_repo, deal_id, &self.validation_config).await?;
        if !status_is_good_or_better(&result.status) {
            return Err(ApplicationError::WinWinWinValidationFailed {
                violations: result
                    .violations
                    .into_iter()
                    .map(|v| v.message)
                    .chain(result.warnings.into_iter().map(|w| w.message))
                    .collect(),
            });
        }
        persist_validation(&*self.deal_repo, deal_id, &result).await?;
        Ok(())
    }

    async fn validate_commit(
        &self,
        deal_id: Uuid,
        acknowledge_warnings: bool,
    ) -> Result<(), ApplicationError> {
        self.deal_repo
            .find_value_distribution_by_deal(deal_id)
            .await?
            .ok_or_else(|| ApplicationError::WinWinWinValidationFailed {
                violations: vec!["value distribution is required before committing".to_string()],
            })?;

        let result = run_validation(&*self.deal_repo, deal_id, &self.validation_config).await?;
        if result.blocked {
            return Err(ApplicationError::WinWinWinValidationFailed {
                violations: result.violations.into_iter().map(|v| v.message).collect(),
            });
        }
        if !status_is_good_or_better(&result.status) {
            return Err(ApplicationError::WinWinWinValidationFailed {
                violations: result
                    .violations
                    .into_iter()
                    .map(|v| v.message)
                    .chain(result.warnings.into_iter().map(|w| w.message))
                    .collect(),
            });
        }
        if !result.warnings.is_empty() && !acknowledge_warnings {
            return Err(ApplicationError::WinWinWinValidationFailed {
                violations: result.warnings.into_iter().map(|w| w.message).collect(),
            });
        }
        persist_validation(&*self.deal_repo, deal_id, &result).await?;
        Ok(())
    }

    async fn generate_agreement(
        &self,
        deal_id: Uuid,
        actor_user_id: Uuid,
        actor_party_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let generator = GenerateAgreement::new(
            self.deal_repo.clone(),
            self.party_repo.clone(),
            self.agreement_repo.clone(),
        );
        generator
            .execute(GenerateAgreementCommand {
                actor_user_id,
                actor_party_id,
                deal_id,
            })
            .await?;
        Ok(())
    }

    async fn require_signed_agreement(&self, deal_id: Uuid) -> Result<(), ApplicationError> {
        let agreement = self
            .agreement_repo
            .find_by_deal_id(deal_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::Validation(vec![
                    "a signed agreement is required before committing".to_string(),
                ])
            })?;

        if agreement.agreement_status != AgreementStatus::Signed {
            return Err(ApplicationError::Validation(vec![format!(
                "agreement status is {}, expected SIGNED before committing",
                agreement.agreement_status.as_str()
            )]));
        }

        Ok(())
    }
}
