use crate::errors::ApplicationError;
use domain::repositories::PartyRepository;
use std::sync::Arc;
use uuid::Uuid;

/// Ensure the actor is the owner of the catalogue item or an admin.
///
/// `owner_party_id` is the party that owns the catalogue item (e.g. `supplier_party_id`).
/// Non-admin actors must act through `actor_party_id` and be an active member of it.
pub async fn require_catalog_owner_or_admin(
    party_repo: &Arc<dyn PartyRepository>,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
    owner_party_id: Uuid,
    is_admin: bool,
) -> Result<(), ApplicationError> {
    if is_admin {
        return Ok(());
    }

    if actor_party_id != owner_party_id {
        return Err(ApplicationError::CatalogAccessDenied);
    }

    let is_member = party_repo
        .is_user_member_of_party(actor_user_id, actor_party_id)
        .await?;

    if !is_member {
        return Err(ApplicationError::Forbidden);
    }

    Ok(())
}

/// Ensure the actor is an active member of the acting party, unless they are an admin.
pub async fn require_party_actor(
    party_repo: &Arc<dyn PartyRepository>,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
    is_admin: bool,
) -> Result<(), ApplicationError> {
    if is_admin {
        return Ok(());
    }

    let is_member = party_repo
        .is_user_member_of_party(actor_user_id, actor_party_id)
        .await?;

    if !is_member {
        return Err(ApplicationError::Forbidden);
    }

    Ok(())
}
