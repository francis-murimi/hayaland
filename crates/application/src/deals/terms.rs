use crate::deals::dto::{CounterTermCommand, ProposeTermCommand, TermActionCommand, TermResult};
use crate::errors::ApplicationError;
use domain::entities::{DealStatus, Term, TermStatus};
use domain::repositories::{DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

fn is_negotiable(status: DealStatus) -> bool {
    matches!(
        status,
        DealStatus::Draft
            | DealStatus::Suggested
            | DealStatus::PendingReview
            | DealStatus::Negotiating
            | DealStatus::OnHold
            | DealStatus::AwaitingParty
    )
}

async fn ensure_actor_can_negotiate(
    deal_repo: &Arc<dyn DealRepository>,
    party_repo: &Arc<dyn PartyRepository>,
    deal_id: Uuid,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
) -> Result<(), ApplicationError> {
    if !party_repo
        .is_user_member_of_party(actor_user_id, actor_party_id)
        .await?
    {
        return Err(ApplicationError::Forbidden);
    }

    let aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await?
        .ok_or(ApplicationError::DealNotFound)?;

    if !is_negotiable(aggregate.deal.deal_status) {
        return Err(ApplicationError::Validation(vec![format!(
            "deal is {} and cannot be negotiated",
            aggregate.deal.deal_status.as_str()
        )]));
    }

    if !deal_repo
        .is_party_participant(deal_id, actor_party_id)
        .await?
    {
        return Err(ApplicationError::Forbidden);
    }

    Ok(())
}

pub(crate) fn map_term_to_result(term: Term) -> TermResult {
    TermResult {
        id: term.id,
        deal_id: term.deal_id,
        proposed_by_party_id: term.proposed_by_party_id,
        term_type: term.term_type,
        term_name: term.term_name,
        description: term.description,
        negotiation_status: term.negotiation_status,
        parent_term_id: term.parent_term_id,
        version: term.version,
        proposed_at: term.proposed_at,
        resolved_at: term.resolved_at,
        is_mandatory: term.is_mandatory,
        resolution: term.resolution,
    }
}

/// Propose a new term on a deal.
#[derive(Clone)]
pub struct ProposeTerm {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl ProposeTerm {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(deal_id = %cmd.deal_id))]
    pub async fn execute(&self, cmd: ProposeTermCommand) -> Result<TermResult, ApplicationError> {
        ensure_actor_can_negotiate(
            &self.deal_repo,
            &self.party_repo,
            cmd.deal_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
        )
        .await?;

        let term = Term::new(
            Uuid::now_v7(),
            cmd.deal_id,
            cmd.actor_party_id,
            cmd.term_type,
            cmd.term_name,
            cmd.description,
            cmd.is_mandatory,
        );

        self.deal_repo.create_term(&term).await?;
        self.deal_repo
            .record_history(cmd.deal_id, "TERM_PROPOSED", Some(cmd.actor_party_id), None)
            .await?;

        info!(term_id = %term.id, "proposed term");
        Ok(map_term_to_result(term))
    }
}

/// Counter an existing term with a new version.
#[derive(Clone)]
pub struct CounterTerm {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl CounterTerm {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(term_id = %cmd.term_id))]
    pub async fn execute(&self, cmd: CounterTermCommand) -> Result<TermResult, ApplicationError> {
        ensure_actor_can_negotiate(
            &self.deal_repo,
            &self.party_repo,
            cmd.deal_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
        )
        .await?;

        let mut existing = self.deal_repo.find_term_by_id(cmd.term_id).await?.ok_or(
            ApplicationError::Validation(vec!["term not found".to_string()]),
        )?;

        let counter = existing.counter(Uuid::now_v7(), cmd.actor_party_id, cmd.description)?;
        existing.negotiation_status = TermStatus::Countered;
        existing.resolved_at = Some(time::OffsetDateTime::now_utc());

        self.deal_repo.update_term(&existing).await?;
        self.deal_repo.create_term(&counter).await?;
        self.deal_repo
            .record_history(
                cmd.deal_id,
                "TERM_COUNTERED",
                Some(cmd.actor_party_id),
                None,
            )
            .await?;

        info!(parent_term = %existing.id, counter_term = %counter.id, "countered term");
        Ok(map_term_to_result(counter))
    }
}

/// Accept a proposed term.
#[derive(Clone)]
pub struct AcceptTerm {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl AcceptTerm {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(term_id = %cmd.term_id))]
    pub async fn execute(&self, cmd: TermActionCommand) -> Result<TermResult, ApplicationError> {
        ensure_actor_can_negotiate(
            &self.deal_repo,
            &self.party_repo,
            cmd.deal_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
        )
        .await?;

        let mut term = self.deal_repo.find_term_by_id(cmd.term_id).await?.ok_or(
            ApplicationError::Validation(vec!["term not found".to_string()]),
        )?;

        term.accept()?;
        self.deal_repo.update_term(&term).await?;
        self.deal_repo
            .record_history(cmd.deal_id, "TERM_ACCEPTED", Some(cmd.actor_party_id), None)
            .await?;

        Ok(map_term_to_result(term))
    }
}

/// Reject a proposed term.
#[derive(Clone)]
pub struct RejectTerm {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl RejectTerm {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(term_id = %cmd.term_id))]
    pub async fn execute(&self, cmd: TermActionCommand) -> Result<TermResult, ApplicationError> {
        ensure_actor_can_negotiate(
            &self.deal_repo,
            &self.party_repo,
            cmd.deal_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
        )
        .await?;

        let mut term = self.deal_repo.find_term_by_id(cmd.term_id).await?.ok_or(
            ApplicationError::Validation(vec!["term not found".to_string()]),
        )?;

        term.reject()?;
        self.deal_repo.update_term(&term).await?;
        self.deal_repo
            .record_history(cmd.deal_id, "TERM_REJECTED", Some(cmd.actor_party_id), None)
            .await?;

        Ok(map_term_to_result(term))
    }
}

/// Withdraw a term (only by the proposing party).
#[derive(Clone)]
pub struct WithdrawTerm {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl WithdrawTerm {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(term_id = %cmd.term_id))]
    pub async fn execute(&self, cmd: TermActionCommand) -> Result<TermResult, ApplicationError> {
        ensure_actor_can_negotiate(
            &self.deal_repo,
            &self.party_repo,
            cmd.deal_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
        )
        .await?;

        let mut term = self.deal_repo.find_term_by_id(cmd.term_id).await?.ok_or(
            ApplicationError::Validation(vec!["term not found".to_string()]),
        )?;

        if term.proposed_by_party_id != cmd.actor_party_id {
            return Err(ApplicationError::Forbidden);
        }

        term.withdraw()?;
        self.deal_repo.update_term(&term).await?;
        self.deal_repo
            .record_history(
                cmd.deal_id,
                "TERM_WITHDRAWN",
                Some(cmd.actor_party_id),
                None,
            )
            .await?;

        Ok(map_term_to_result(term))
    }
}

/// List terms for a deal.
#[derive(Clone)]
pub struct ListTerms {
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl ListTerms {
    pub fn new(deal_repo: Arc<dyn DealRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self), fields(deal_id = %deal_id))]
    pub async fn execute(
        &self,
        deal_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        is_admin: bool,
    ) -> Result<Vec<TermResult>, ApplicationError> {
        let aggregate = self
            .deal_repo
            .find_aggregate_by_id(deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !is_admin {
            let visible_party_ids: Vec<Uuid> = aggregate
                .participations
                .iter()
                .map(|p| p.party_id)
                .collect();

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

        let terms = self.deal_repo.find_terms_by_deal(deal_id).await?;
        Ok(terms.into_iter().map(map_term_to_result).collect())
    }
}
