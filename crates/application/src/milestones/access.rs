use crate::errors::ApplicationError;
use domain::entities::DealStatus;
use domain::repositories::{DealRepository, PartyRepository};
use std::sync::Arc;
use uuid::Uuid;

pub async fn ensure_participant(
    party_repo: &Arc<dyn PartyRepository>,
    deal_repo: &Arc<dyn DealRepository>,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
    deal_id: Uuid,
) -> Result<(), ApplicationError> {
    if !party_repo
        .is_user_member_of_party(actor_user_id, actor_party_id)
        .await?
    {
        return Err(ApplicationError::Forbidden);
    }

    if !deal_repo
        .is_party_participant(deal_id, actor_party_id)
        .await?
    {
        return Err(ApplicationError::DealAccessDenied);
    }

    Ok(())
}

pub fn allow_milestone_mutations(status: DealStatus) -> Result<(), ApplicationError> {
    match status {
        DealStatus::Committed | DealStatus::Executing => Ok(()),
        _ => Err(ApplicationError::Validation(vec![
            "milestones can only be modified while the deal is committed or executing".to_string(),
        ])),
    }
}
