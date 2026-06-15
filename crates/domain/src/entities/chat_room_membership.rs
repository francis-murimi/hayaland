use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Role of a member within a chatroom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatRoomMemberRole {
    Member,
    Moderator,
}

impl ChatRoomMemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatRoomMemberRole::Member => "MEMBER",
            ChatRoomMemberRole::Moderator => "MODERATOR",
        }
    }
}

impl TryFrom<&str> for ChatRoomMemberRole {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "MEMBER" => Ok(ChatRoomMemberRole::Member),
            "MODERATOR" => Ok(ChatRoomMemberRole::Moderator),
            _ => Err(DomainError::InvalidChatRoomMemberRole {
                message: format!("unknown chat room member role: {value}"),
            }),
        }
    }
}

/// A user's or party's membership in a chatroom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRoomMembership {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub member_role: ChatRoomMemberRole,
    pub joined_at: OffsetDateTime,
}

impl ChatRoomMembership {
    pub fn for_user(
        id: Uuid,
        room_id: Uuid,
        user_id: Uuid,
        member_role: ChatRoomMemberRole,
    ) -> Self {
        Self {
            id,
            room_id,
            user_id: Some(user_id),
            party_id: None,
            member_role,
            joined_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn for_party(
        id: Uuid,
        room_id: Uuid,
        party_id: Uuid,
        member_role: ChatRoomMemberRole,
    ) -> Self {
        Self {
            id,
            room_id,
            user_id: None,
            party_id: Some(party_id),
            member_role,
            joined_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn is_moderator(&self) -> bool {
        matches!(self.member_role, ChatRoomMemberRole::Moderator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_role_from_str() {
        assert_eq!(
            ChatRoomMemberRole::try_from("MODERATOR").unwrap(),
            ChatRoomMemberRole::Moderator
        );
        assert!(ChatRoomMemberRole::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn user_membership_stores_user_id() {
        let user_id = Uuid::now_v7();
        let membership = ChatRoomMembership::for_user(
            Uuid::now_v7(),
            Uuid::now_v7(),
            user_id,
            ChatRoomMemberRole::Member,
        );
        assert_eq!(membership.user_id, Some(user_id));
        assert!(membership.party_id.is_none());
        assert!(!membership.is_moderator());
    }

    #[test]
    fn moderator_membership_is_moderator() {
        let membership = ChatRoomMembership::for_user(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            ChatRoomMemberRole::Moderator,
        );
        assert!(membership.is_moderator());
    }
}
