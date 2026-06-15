use crate::errors::ApplicationError;
use domain::entities::{Conversation, ConversationType, Message, RecipientType};
use domain::repositories::{ChatRoomRepository, DealRepository, PartyRepository};

use uuid::Uuid;

pub fn has_scope(scopes: &[String], scope: &str) -> bool {
    scopes.iter().any(|s| s == scope || s == "admin:*")
}

pub fn is_admin_scope(scopes: &[String]) -> bool {
    has_scope(scopes, "admin:messages")
}

/// Determine whether the actor is allowed to view a message.
pub async fn is_message_visible_to_actor(
    msg: &Message,
    actor_user_id: Uuid,
    actor_party_id: Option<Uuid>,
    is_admin: bool,
    party_repo: &dyn PartyRepository,
    deal_repo: &dyn DealRepository,
    room_repo: &dyn ChatRoomRepository,
) -> Result<bool, ApplicationError> {
    if is_admin || msg.sender_user_id == actor_user_id {
        return Ok(true);
    }

    match msg.recipient_type {
        RecipientType::User => Ok(msg.recipient_user_id == Some(actor_user_id)),
        RecipientType::Party => {
            let party_id = match msg.recipient_party_id {
                Some(id) => id,
                None => return Ok(false),
            };
            if actor_party_id == Some(party_id) {
                return Ok(true);
            }
            Ok(party_repo
                .is_user_member_of_party(actor_user_id, party_id)
                .await?)
        }
        RecipientType::PartyMembers => {
            let party_id = match msg.sender_party_id {
                Some(id) => id,
                None => return Ok(false),
            };
            Ok(party_repo
                .is_user_member_of_party(actor_user_id, party_id)
                .await?)
        }
        RecipientType::Deal => {
            let deal_id = match msg.recipient_deal_id {
                Some(id) => id,
                None => return Ok(false),
            };
            let participations = deal_repo.find_participations_by_deal(deal_id).await?;
            for p in participations {
                if actor_party_id == Some(p.party_id)
                    && party_repo
                        .is_user_member_of_party(actor_user_id, p.party_id)
                        .await?
                {
                    return Ok(true);
                }
                if party_repo
                    .is_user_member_of_party(actor_user_id, p.party_id)
                    .await?
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        RecipientType::Room => {
            let room_id = match msg.recipient_room_id {
                Some(id) => id,
                None => return Ok(false),
            };
            if let Some(party_id) = actor_party_id {
                if room_repo.is_party_in_room(room_id, &[party_id]).await? {
                    return Ok(true);
                }
            }
            let party_ids = actor_party_id.into_iter().collect::<Vec<_>>();
            Ok(room_repo
                .is_user_in_room(room_id, actor_user_id, &party_ids)
                .await?)
        }
        RecipientType::AdminBroadcast => Ok(true),
    }
}

/// Determine whether the actor is allowed to access a whole conversation.
pub async fn is_conversation_visible_to_actor(
    conversation: &Conversation,
    actor_user_id: Uuid,
    actor_party_id: Option<Uuid>,
    is_admin: bool,
    party_repo: &dyn PartyRepository,
    deal_repo: &dyn DealRepository,
    room_repo: &dyn ChatRoomRepository,
) -> Result<bool, ApplicationError> {
    if is_admin {
        return Ok(true);
    }
    match conversation.conversation_type {
        ConversationType::DirectUser => Ok(conversation.user_a_id == Some(actor_user_id)
            || conversation.user_b_id == Some(actor_user_id)),
        ConversationType::DirectParty => {
            Ok(conversation.party_a_id == actor_party_id
                || conversation.party_b_id == actor_party_id)
        }
        ConversationType::PartyMembers => {
            let party_id = match conversation.party_id {
                Some(id) => id,
                None => return Ok(false),
            };
            Ok(party_repo
                .is_user_member_of_party(actor_user_id, party_id)
                .await?)
        }
        ConversationType::Deal => {
            let deal_id = match conversation.deal_id {
                Some(id) => id,
                None => return Ok(false),
            };
            let participations = deal_repo.find_participations_by_deal(deal_id).await?;
            for p in participations {
                if actor_party_id == Some(p.party_id)
                    && party_repo
                        .is_user_member_of_party(actor_user_id, p.party_id)
                        .await?
                {
                    return Ok(true);
                }
                if party_repo
                    .is_user_member_of_party(actor_user_id, p.party_id)
                    .await?
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        ConversationType::Room => {
            let room_id = match conversation.room_id {
                Some(id) => id,
                None => return Ok(false),
            };
            let party_ids: Vec<_> = actor_party_id.into_iter().collect();
            Ok(room_repo
                .is_user_in_room(room_id, actor_user_id, &party_ids)
                .await?)
        }
        ConversationType::AdminBroadcast => Ok(true),
    }
}
