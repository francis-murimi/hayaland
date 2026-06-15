use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// The messaging context that groups a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationType {
    DirectUser,
    DirectParty,
    PartyMembers,
    Deal,
    Room,
    AdminBroadcast,
}

impl ConversationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationType::DirectUser => "DIRECT_USER",
            ConversationType::DirectParty => "DIRECT_PARTY",
            ConversationType::PartyMembers => "PARTY_MEMBERS",
            ConversationType::Deal => "DEAL",
            ConversationType::Room => "ROOM",
            ConversationType::AdminBroadcast => "ADMIN_BROADCAST",
        }
    }
}

impl TryFrom<&str> for ConversationType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DIRECT_USER" => Ok(ConversationType::DirectUser),
            "DIRECT_PARTY" => Ok(ConversationType::DirectParty),
            "PARTY_MEMBERS" => Ok(ConversationType::PartyMembers),
            "DEAL" => Ok(ConversationType::Deal),
            "ROOM" => Ok(ConversationType::Room),
            "ADMIN_BROADCAST" => Ok(ConversationType::AdminBroadcast),
            _ => Err(DomainError::InvalidConversationType {
                message: format!("unknown conversation type: {value}"),
            }),
        }
    }
}

/// A thread that groups related messages by context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub conversation_type: ConversationType,
    pub user_a_id: Option<Uuid>,
    pub user_b_id: Option<Uuid>,
    pub party_a_id: Option<Uuid>,
    pub party_b_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub deal_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    pub title: Option<String>,
    pub last_message_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

impl Conversation {
    pub fn new_direct_user(id: Uuid, user_a_id: Uuid, user_b_id: Uuid) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            conversation_type: ConversationType::DirectUser,
            user_a_id: Some(user_a_id),
            user_b_id: Some(user_b_id),
            party_a_id: None,
            party_b_id: None,
            party_id: None,
            deal_id: None,
            room_id: None,
            title: None,
            last_message_at: now,
            created_at: now,
        }
    }

    pub fn new_direct_party(id: Uuid, party_a_id: Uuid, party_b_id: Uuid) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            conversation_type: ConversationType::DirectParty,
            user_a_id: None,
            user_b_id: None,
            party_a_id: Some(party_a_id),
            party_b_id: Some(party_b_id),
            party_id: None,
            deal_id: None,
            room_id: None,
            title: None,
            last_message_at: now,
            created_at: now,
        }
    }

    pub fn new_party_members(id: Uuid, party_id: Uuid, title: Option<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            conversation_type: ConversationType::PartyMembers,
            user_a_id: None,
            user_b_id: None,
            party_a_id: None,
            party_b_id: None,
            party_id: Some(party_id),
            deal_id: None,
            room_id: None,
            title,
            last_message_at: now,
            created_at: now,
        }
    }

    pub fn new_deal(id: Uuid, deal_id: Uuid, title: Option<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            conversation_type: ConversationType::Deal,
            user_a_id: None,
            user_b_id: None,
            party_a_id: None,
            party_b_id: None,
            party_id: None,
            deal_id: Some(deal_id),
            room_id: None,
            title,
            last_message_at: now,
            created_at: now,
        }
    }

    pub fn new_room(id: Uuid, room_id: Uuid, title: Option<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            conversation_type: ConversationType::Room,
            user_a_id: None,
            user_b_id: None,
            party_a_id: None,
            party_b_id: None,
            party_id: None,
            deal_id: None,
            room_id: Some(room_id),
            title,
            last_message_at: now,
            created_at: now,
        }
    }

    pub fn touch_last_message_at(&mut self) {
        self.last_message_at = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_type_from_str() {
        assert_eq!(
            ConversationType::try_from("DIRECT_USER").unwrap(),
            ConversationType::DirectUser
        );
        assert_eq!(
            ConversationType::try_from("ROOM").unwrap(),
            ConversationType::Room
        );
        assert!(ConversationType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn direct_user_conversation_has_users() {
        let a = Uuid::now_v7();
        let b = Uuid::now_v7();
        let conv = Conversation::new_direct_user(Uuid::now_v7(), a, b);
        assert_eq!(conv.conversation_type, ConversationType::DirectUser);
        assert_eq!(conv.user_a_id, Some(a));
        assert_eq!(conv.user_b_id, Some(b));
    }

    #[test]
    fn deal_conversation_has_deal_id() {
        let deal_id = Uuid::now_v7();
        let conv = Conversation::new_deal(Uuid::now_v7(), deal_id, Some("Deal chat".into()));
        assert_eq!(conv.deal_id, Some(deal_id));
        assert_eq!(conv.title, Some("Deal chat".into()));
    }
}
