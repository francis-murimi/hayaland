use crate::deals::dto::ValidateDealResult;
use crate::errors::ApplicationError;
use domain::entities::{Deal, Term, ValueDistribution};
use domain::repositories::{DealRepository, PartyRepository};
use domain::services::{
    PartyValidationSnapshot, ValidationConfig, ValidationInput, WinWinWinValidator,
};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Query used to authorize validation of a deal.
#[derive(Debug, Clone)]
pub struct ValidateDealQuery {
    pub actor_user_id: Uuid,
    pub actor_party_id: Option<Uuid>,
    pub is_admin: bool,
}

/// Run Win-Win-Win validation for a deal and persist the result.
#[derive(Clone)]
pub struct ValidateDeal {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
    validation_config: ValidationConfig,
}

impl ValidateDeal {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
        validation_config: ValidationConfig,
    ) -> Self {
        Self {
            deal_repo,
            party_repo,
            validation_config,
        }
    }

    #[instrument(skip(self, query), fields(deal_id = %deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        query: ValidateDealQuery,
    ) -> Result<ValidateDealResult, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !query.is_admin {
            let visible_party_ids: Vec<Uuid> = aggregate
                .participations
                .iter()
                .map(|p| p.party_id)
                .collect();

            let is_member = query
                .actor_party_id
                .map(|pid| visible_party_ids.contains(&pid))
                .unwrap_or(false);

            if !is_member {
                let mut member_of_any = false;
                for pid in &visible_party_ids {
                    if self
                        .party_repo
                        .is_user_member_of_party(query.actor_user_id, *pid)
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

        let result = run_validation(&*self.deal_repo, deal_id, &self.validation_config).await?;
        persist_validation(&*self.deal_repo, deal_id, &result).await?;
        info!(deal_id = %deal_id, score = %result.score, status = ?result.status, "validated deal");
        Ok(map_result(result))
    }
}

pub(crate) async fn run_validation(
    deal_repo: &dyn DealRepository,
    deal_id: Uuid,
    config: &ValidationConfig,
) -> Result<domain::services::ValidationResult, ApplicationError> {
    let aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await?
        .ok_or(ApplicationError::DealNotFound)?;

    let value_distribution = deal_repo
        .find_value_distribution_by_deal(deal_id)
        .await?
        .ok_or_else(|| ApplicationError::WinWinWinValidationFailed {
            violations: vec!["value distribution is required before validation".to_string()],
        })?;

    let terms = deal_repo.find_terms_by_deal(deal_id).await?;
    let input = build_validation_input(&aggregate.deal, &value_distribution, &terms);
    Ok(WinWinWinValidator::validate(&input, config))
}

pub(crate) async fn persist_validation(
    deal_repo: &dyn DealRepository,
    deal_id: Uuid,
    result: &domain::services::ValidationResult,
) -> Result<(), ApplicationError> {
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await?
        .ok_or(ApplicationError::DealNotFound)?;
    let now = time::OffsetDateTime::now_utc();
    aggregate.deal.validation_score = Some(result.score);
    aggregate.deal.win_win_win_validated = !result.blocked;
    aggregate.deal.validation_checked_at = Some(now);
    aggregate.deal.validation_result = Some(serde_json::to_value(result).unwrap_or_default());
    aggregate.deal.updated_at = now;
    deal_repo.update(&aggregate.deal).await?;
    Ok(())
}

pub(crate) fn build_validation_input(
    deal: &Deal,
    value_distribution: &ValueDistribution,
    terms: &[Term],
) -> ValidationInput {
    let all_mandatory_terms_accepted = terms
        .iter()
        .filter(|t| t.is_mandatory)
        .all(|t| t.negotiation_status == domain::entities::TermStatus::Accepted);

    ValidationInput {
        value_distribution: value_distribution.clone(),
        supplier: PartyValidationSnapshot::default(),
        consumer: PartyValidationSnapshot::default(),
        enhancer: PartyValidationSnapshot::default(),
        all_mandatory_terms_accepted,
        market_benchmark_premium: deal.validation_score,
    }
}

pub(crate) fn status_is_good_or_better(status: &domain::services::ValidationStatus) -> bool {
    matches!(
        status,
        domain::services::ValidationStatus::Excellent | domain::services::ValidationStatus::Good
    )
}

pub(crate) fn status_to_string(status: domain::services::ValidationStatus) -> String {
    match status {
        domain::services::ValidationStatus::Excellent => "EXCELLENT".to_string(),
        domain::services::ValidationStatus::Good => "GOOD".to_string(),
        domain::services::ValidationStatus::Fair => "FAIR".to_string(),
        domain::services::ValidationStatus::Poor => "POOR".to_string(),
        domain::services::ValidationStatus::Blocked => "BLOCKED".to_string(),
    }
}

pub(crate) fn map_result(result: domain::services::ValidationResult) -> ValidateDealResult {
    ValidateDealResult {
        score: result.score,
        status: status_to_string(result.status),
        blocked: result.blocked,
        violations: result.violations,
        warnings: result.warnings,
        party_feedback: result.party_feedback,
    }
}
